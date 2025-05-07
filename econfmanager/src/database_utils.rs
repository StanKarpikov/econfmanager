use rusqlite::{Connection, OpenFlags, ToSql, backup::Backup, params};
use std::time::Duration;
use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[allow(unused_imports)]
use log::{debug, error, info, warn};

use crate::{
    config::Config,
    generated::{PARAMETER_DATA, ParameterId},
    schema::{Parameter, ParameterValue},
};

const TABLE_NAME: &str = "parameters";

#[derive(Default)]
pub(crate) struct DatabaseManager {
    database_path: String,
    saved_database_path: String,
    default_data_folder: String,
    last_update_timestamp: f64,
}

pub struct DbConnection {
    conn: Option<Connection>,
}

impl DbConnection {
    pub fn new(
        database_path: &String,
        write_required: bool,
        create_required: bool,
    ) -> Result<Self, Box<dyn Error>> {
        let flags = if write_required {
            let mut f = OpenFlags::SQLITE_OPEN_READ_WRITE;
            if create_required {
                f.insert(OpenFlags::SQLITE_OPEN_CREATE);
            }
            f
        } else {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        };

        let mut conn = match Connection::open_with_flags(&database_path, flags) {
            Ok(conn) => {
                let _ = conn.busy_timeout(std::time::Duration::from_millis(300));
                conn
            }
            Err(e) => {
                return Err(format!("Failed to open connection: {}", e).into());
            }
        };
        debug!("> DB connection opened with flags {:?}", flags);

        if create_required {
            let sql = format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    key INTEGER UNIQUE PRIMARY KEY,
                    value BLOB,
                    timestamp REAL
                ) WITHOUT ROWID;",
                TABLE_NAME
            );
            let tx = conn.transaction()?;
            tx.execute_batch(&sql)?;
            tx.commit()?;

            conn.pragma_update(None, "locking_mode", "NORMAL")?;
            conn.pragma_update(None, "journal_mode", "WAL")?;

            // TODO: Optional: needs testing
            // conn.execute_batch(&format!("PRAGMA auto_vacuum = {};", "INCREMENTAL"))?;
            conn.execute("VACUUM", [])?;

            conn.pragma_update(None, "wal_autocheckpoint", "1000")?; // Pages
            conn.pragma_update(None, "synchronous", "NORMAL")?;
            conn.pragma_update(None, "busy_timeout", "10000")?; // 10 second timeout

            info!("Parameters database created");
        }

        Ok(Self { conn: Some(conn) })
    }

    pub fn conn(&self) -> &Connection {
        self.conn
            .as_ref()
            .expect("Connection is always Some while DbConnection exists")
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        self.conn
            .as_mut()
            .expect("Connection is always Some while DbConnection exists")
    }
}

impl Drop for DbConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let _ = conn.close();
            debug!("< DB connection closed");
        } else {
            warn!("< Drop was called, but DB connection not closed");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status<T> {
    StatusOkChanged(T),
    StatusOkNotChanged(T),
    StatusOkNotChecked(T),
    StatusOkOverflowFixed(T),
    StatusErrorNotAccepted(T),
    StatusErrorFailed,
}

impl<T: fmt::Display> fmt::Display for Status<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::StatusOkChanged(value) => write!(f, "OK (changed): {}", value),
            Status::StatusOkNotChanged(value) => write!(f, "OK (not changed): {}", value),
            Status::StatusOkNotChecked(value) => write!(f, "OK (not checked): {}", value),
            Status::StatusOkOverflowFixed(value) => write!(f, "OK (overflow fixed): {}", value),
            Status::StatusErrorNotAccepted(value) => write!(f, "Error (not accepted): {}", value),
            Status::StatusErrorFailed => write!(f, "Error (operation failed)"),
        }
    }
}

impl DatabaseManager {
    /******************************************************************************
     * PRIVATE FUNCTIONS
     ******************************************************************************/

    /// Returns current timestamp with seconds and milliseconds as a floating-point number
    /// (e.g. 1712345678.456 for 456 milliseconds past the second)
    fn get_timestamp() -> f64 {
        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let seconds = duration.as_secs() as f64;
        let milliseconds = (duration.subsec_millis() as f64) / 1000.0;

        seconds + milliseconds
    }

    fn copy_database(
        source_path: &Path,
        backup_path: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let src_conn = Connection::open(source_path)?;
        let mut dst_conn = Connection::open(backup_path)?;

        let backup = Backup::new(&src_conn, &mut dst_conn)?;
        Ok(backup.run_to_completion(100, Duration::from_millis(250), None)?)
    }

    fn copy_database_with_filter(
        source_path: &Path,
        backup_path: &Path,
        filter: &dyn Fn(&String) -> bool,
    ) -> Result<(), Box<dyn Error>> {
        let src_conn = Connection::open(source_path)?;
        let dst_conn = Connection::open(backup_path)?;

        dst_conn.execute(
            &format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    key INTEGER UNIQUE PRIMARY KEY,
                    value BLOB,
                    timestamp REAL
                ) WITHOUT ROWID;",
                TABLE_NAME
            ),
            [],
        )?;

        let mut src_stmt =
            src_conn.prepare(&format!("SELECT key, value, timestamp FROM {}", TABLE_NAME))?;
        let mut rows = src_stmt.query([])?;

        let mut dst_stmt = dst_conn.prepare(&format!(
            "INSERT INTO {} (key, value, timestamp) VALUES (?1, ?2, ?3)",
            TABLE_NAME
        ))?;

        while let Some(row) = rows.next()? {
            let key = row.get(0).unwrap_or("".to_string());
            if filter(&key) {
                let value: rusqlite::types::Value = row.get(1)?;
                let timestamp: f64 = std::f64::MAX;
                dst_stmt.execute(params![key, value, timestamp])?;
            }
        }

        Ok(())
    }

    /******************************************************************************
     * PUBLIC FUNCTIONS
     ******************************************************************************/

    pub(crate) fn drop_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Deleting database");
    
        let result = {
            let db = DbConnection::new(&self.database_path, true, false)?;
            db.conn().execute(&format!("DROP TABLE {};", TABLE_NAME), [])?;
            db.conn().execute("VACUUM", [])
        };
    
        let _ = DbConnection::new(&self.database_path, true, true)?;

        result?;
        Ok(())
    }

    pub(crate) fn load_database(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Loading database");
        if let Err(error) = self.drop_database() {
            error!("Could not drop the database: {}", error);
        }
        info!("Copying database");
        Self::copy_database(
            Path::new(&self.saved_database_path),
            Path::new(&self.database_path),
        )
    }

    pub(crate) fn save_database(
        &self,
        filter: &dyn Fn(&String) -> bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Saving database");
        Self::copy_database_with_filter(
            Path::new(&self.database_path),
            Path::new(&self.saved_database_path),
            &filter,
        )?;
        Ok(())
    }

    pub(crate) fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let database_manager = Self {
            database_path: config.database_path.clone(),
            saved_database_path: config.saved_database_path.clone(),
            last_update_timestamp: 0.0,
            default_data_folder: config.default_data_folder.clone(),
        };

        match fs::metadata(&database_manager.database_path) {
            Ok(metadata) if metadata.is_file() => {
                info!("Database exists, continue");
            }
            Ok(_) => {
                error!(
                    "Database file {} exists but is not a file",
                    database_manager.database_path
                );
                return Err(format!(
                    "Database file {} exists but is not a file",
                    database_manager.database_path
                )
                .into());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                info!("Database doesn't exist, load");
                database_manager.load_database()?;
            }
            Err(e) => {
                error!(
                    "Error checking database file {}: {}",
                    database_manager.database_path, e
                );
                return Err(format!(
                    "Error checking database file {}: {}",
                    database_manager.database_path, e
                )
                .into());
            }
        }

        DbConnection::new(&database_manager.database_path, true, true)?;
        info!("Database manager initialised");
        Ok(database_manager)
    }

    #[allow(unused)]
    pub(crate) fn set_sqlite_version(&self, version: u32) -> Result<(), Box<dyn Error>> {
        let db = DbConnection::new(&self.database_path, false, false)?;

        db.conn().pragma_update(None, "user_version", version)?;

        // let user_version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        // println!("Database user_version set to: {}", user_version);

        Ok(())
    }

    fn db_to_bool(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => Ok(ParameterValue::ValBool(i != 0)),
            rusqlite::types::Value::Real(f) => Ok(ParameterValue::ValBool(f != 0.0)),
            _ => {
                return Err("".into());
            }
        }
    }

    fn db_to_i32(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => Ok(ParameterValue::ValI32(i as i32)),
            rusqlite::types::Value::Real(f) => Ok(ParameterValue::ValI32(f as i32)),
            _ => {
                return Err("".into());
            }
        }
    }

    fn db_to_u32(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => Ok(ParameterValue::ValU32(i as u32)),
            rusqlite::types::Value::Real(f) => Ok(ParameterValue::ValU32(f as u32)),
            _ => {
                return Err("".into());
            }
        }
    }

    fn db_to_i64(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => Ok(ParameterValue::ValI64(i as i64)),
            rusqlite::types::Value::Real(f) => Ok(ParameterValue::ValI64(f as i64)),
            _ => {
                return Err("".into());
            }
        }
    }

    fn db_to_u64(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => Ok(ParameterValue::ValU64(i as u64)),
            rusqlite::types::Value::Real(f) => Ok(ParameterValue::ValU64(f as u64)),
            _ => {
                return Err("".into());
            }
        }
    }

    fn db_to_f32(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => Ok(ParameterValue::ValF32(i as f32)),
            rusqlite::types::Value::Real(f) => Ok(ParameterValue::ValF32(f as f32)),
            _ => {
                return Err("".into());
            }
        }
    }

    fn db_to_f64(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => Ok(ParameterValue::ValF64(i as f64)),
            rusqlite::types::Value::Real(f) => Ok(ParameterValue::ValF64(f as f64)),
            _ => {
                return Err("".into());
            }
        }
    }

    fn db_to_string(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Text(string) => Ok(ParameterValue::ValString(string.into())),
            _ => {
                return Err("".into());
            }
        }
    }

    fn db_to_blob(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Blob(blob) => Ok(ParameterValue::ValBlob(blob)),
            _ => {
                return Err("".into());
            }
        }
    }

    fn get_default_value(
        &self,
        parameter_def: &Parameter,
    ) -> Result<ParameterValue, rusqlite::Error> {
        match parameter_def.value_default {
            ParameterValue::ValPath(p) => {
                let full_path = PathBuf::from(self.default_data_folder.clone()).join(p);
                let bytes = fs::read(full_path)
                    .map_err(|e| {
                        let err = format!("Error reading file {} with default data: {}", p, e);
                        error!("{}", err);
                        err
                    })
                    .unwrap_or(vec![]);
                Ok(ParameterValue::ValBlob(bytes))
            }
            _ => Ok(parameter_def.value_default.clone()),
        }
    }

    pub(crate) fn read_or_create(&self, id: ParameterId) -> Result<ParameterValue, Box<dyn Error>> {
        let db = DbConnection::new(&self.database_path, false, false)?;

        let sql = format!("SELECT value FROM {} WHERE key = ?", TABLE_NAME);
        let mut stmt = match db.conn().prepare(&sql) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to prepare statement: {}", e);
                return Err(format!("Failed to prepare statement: {}", e).into());
            }
        };

        let parameter_def = &PARAMETER_DATA[id as usize];
        let key = parameter_def.name_id;
        let result = match stmt.query_row(params![key], |row| {
            let sql_value: rusqlite::types::Value = row.get(0)?;
            let data_type = sql_value.data_type();

            let value_result = match parameter_def.value_type {
                ParameterValue::ValBool(_) => Self::db_to_bool(sql_value),
                ParameterValue::ValI32(_) => Self::db_to_i32(sql_value),
                ParameterValue::ValU32(_) => Self::db_to_u32(sql_value),
                ParameterValue::ValI64(_) => Self::db_to_i64(sql_value),
                ParameterValue::ValU64(_) => Self::db_to_u64(sql_value),
                ParameterValue::ValF32(_) => Self::db_to_f32(sql_value),
                ParameterValue::ValF64(_) => Self::db_to_f64(sql_value),
                ParameterValue::ValString(_) => Self::db_to_string(sql_value),
                ParameterValue::ValBlob(_) => Self::db_to_blob(sql_value),
                ParameterValue::ValPath(_) => todo!(),
            };

            match value_result {
                Ok(value) => Ok(value),
                Err(_) => {
                    warn!(
                        "Type mismatch for [{}], using default (SQL is {}, required is {})",
                        key, data_type, parameter_def.value_type
                    );
                    self.get_default_value(parameter_def)
                }
            }
        }) {
            Ok(val) => Ok(val),
            Err(e) => {
                error!("Error reading parameter {}: {}", key, e);
                self.get_default_value(parameter_def)
            }
        };
        Ok(result?)
    }

    pub fn write(
        &self,
        id: ParameterId,
        value: ParameterValue,
        force: bool,
    ) -> Result<Status<ParameterValue>, Box<dyn Error>> {
        // validate(id, &value)?;

        // Check if values are equal (unless forced)
        if !force {
            match self.read_or_create(id) {
                Ok(current) => {
                    if current == value {
                        debug!("Values are equal, skip writing");
                        return Ok(Status::StatusOkNotChanged(value));
                    }
                }
                Err(e) => error!("Error reading current value: {}", e),
            };
        }
        debug!("Write to DB: {}", value);

        let db = DbConnection::new(&self.database_path, true, false)?;

        let sql = format!(
            "INSERT OR REPLACE INTO {} (key, value, timestamp) VALUES (?,?,?);",
            TABLE_NAME
        );

        let mut stmt = db.conn.as_ref().unwrap().prepare(&sql)?;

        let parameter_def = &PARAMETER_DATA[id as usize];
        stmt.execute(params![
            parameter_def.name_id,
            match &value {
                ParameterValue::ValBool(v) => v.to_sql()?,
                ParameterValue::ValI32(v) => v.to_sql()?,
                ParameterValue::ValU32(v) => v.to_sql()?,
                ParameterValue::ValI64(v) => v.to_sql()?,
                ParameterValue::ValU64(v) => v.to_sql()?,
                ParameterValue::ValF32(v) => v.to_sql()?,
                ParameterValue::ValF64(v) => v.to_sql()?,
                ParameterValue::ValString(v) => v.to_sql()?,
                ParameterValue::ValBlob(v) => v.to_sql()?,
                ParameterValue::ValPath(_) => todo!(),
            },
            Self::get_timestamp(),
        ])?;

        Ok(Status::StatusOkChanged(value))
    }

    pub fn update(&mut self) -> Result<Vec<ParameterId>, Box<dyn Error>> {
        let sql = format!("SELECT key FROM {} WHERE timestamp >= ?", TABLE_NAME);
        let check_start = Self::get_timestamp();
        let mut pending_callbacks: Vec<ParameterId> = Vec::new();

        let db = DbConnection::new(&self.database_path, false, false)?;

        let mut stmt = db.conn().prepare(&sql)?;
        let mut rows = stmt.query(params![self.last_update_timestamp])?;

        while let Some(row) = rows.next()? {
            let key = row.get::<usize, String>(0)?;

            let id = PARAMETER_DATA
                .iter()
                .position(|pm| pm.name_id == key)
                .expect("Parameter not found");

            let pm_id = match ParameterId::try_from(id) {
                Ok(param) => param,
                Err(_) => {
                    return Err(format!("Invalid parameter value: {}", id).into());
                }
            };
            debug!("Parameter {} {} updated", key, pm_id as usize);
            pending_callbacks.push(pm_id);
        }

        self.last_update_timestamp = check_start;

        Ok(pending_callbacks)
    }
}
