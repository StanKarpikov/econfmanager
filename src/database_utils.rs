use std::{error::Error, time::{SystemTime, UNIX_EPOCH}};
use rusqlite::{params, Connection, OpenFlags, ToSql};

use crate::{configfile::Config, interface::generated::{ParameterId, PARAMETER_DATA}, schema::ParameterValue};

pub(crate) struct DatabaseManager {
    database_path: String,
    db_opened: bool,
    last_update_timestamp: f64
}

pub struct DbConnection {
    conn: Option<Connection>
}

impl DbConnection {
    pub fn new(database_path: &String, write_required: bool, create_required: bool) -> Result<Self, Box<dyn Error>> {
        let flags = if write_required {
            let mut f = OpenFlags::SQLITE_OPEN_READ_WRITE;
            if create_required {
                f.insert(OpenFlags::SQLITE_OPEN_CREATE);
            }
            f
        } else {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        };

        let conn = match Connection::open_with_flags(&database_path, flags) {
            Ok(conn) => {
                conn.busy_timeout(std::time::Duration::from_millis(300));
                conn
            },
            Err(e) => {
                return Err(format!("Failed to open connection: {}", e).into());
            }
        };

        if create_required {
            conn.pragma_update(None, "locking_mode", "NORMAL")?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
        
            // TODO: Optional: needs testing
            conn.pragma_update(None, "wal_autocheckpoint", "1000")?;  // Pages
            conn.pragma_update(None, "synchronous", "NORMAL")?;
            conn.pragma_update(None, "busy_timeout", "10000")?;  // 10 second timeout
        }

        Ok(Self{conn: Some(conn) })
    }

    pub fn conn(&self) -> &Connection {
        self.conn.as_ref().expect("Connection is always Some while DbConnection exists")
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        self.conn.as_mut().expect("Connection is always Some while DbConnection exists")
    }
}

impl Drop for DbConnection {
    fn drop(&mut self) {
        if let Some(conn) = self.conn.take() {
            let _ = conn.close();
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
    
    /******************************************************************************
     * PUBLIC FUNCTIONS
     ******************************************************************************/
    
    pub(crate) fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> { 
        let database_manager = Self { database_path: config.database_path, db_opened: false, last_update_timestamp: 0.0 };
        DbConnection::new(&database_manager.database_path, true, true)?;
        Ok(database_manager)
    }

    pub(crate) fn set_sqlite_version(&self, version: u32) -> Result<(), Box<dyn Error>> {
        let db = DbConnection::new(&self.database_path, false, false)?;
        
        db.conn().pragma_update(None, "user_version", version)?;
    
        // let user_version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        // println!("Database user_version set to: {}", user_version);
    
        Ok(())
    }
    
    fn db_to_bool(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => {
                Ok(ParameterValue::ValBool(i != 0))
            },
            rusqlite::types::Value::Real(f) => {
                Ok(ParameterValue::ValBool(f != 0.0))
            },
            _ => {
                return Err("".into());
            },
        }
    }

    fn db_to_i32(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => {
                Ok(ParameterValue::ValI32(i as i32))
            },
            rusqlite::types::Value::Real(f) => {
                Ok(ParameterValue::ValI32(f as i32))
            },
            _ => {
                return Err("".into());
            },
        }
    }

    fn db_to_u32(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => {
                Ok(ParameterValue::ValU32(i as u32))
            },
            rusqlite::types::Value::Real(f) => {
                Ok(ParameterValue::ValU32(f as u32))
            },
            _ => {
                return Err("".into());
            },
        }
    }

    fn db_to_i64(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => {
                Ok(ParameterValue::ValI64(i as i64))
            },
            rusqlite::types::Value::Real(f) => {
                Ok(ParameterValue::ValI64(f as i64))
            },
            _ => {
                return Err("".into());
            },
        }
    }

    fn db_to_u64(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => {
                Ok(ParameterValue::ValU64(i as u64))
            },
            rusqlite::types::Value::Real(f) => {
                Ok(ParameterValue::ValU64(f as u64))
            },
            _ => {
                return Err("".into());
            },
        }
    }

    fn db_to_f32(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => {
                Ok(ParameterValue::ValF32(i as f32))
            },
            rusqlite::types::Value::Real(f) => {
                Ok(ParameterValue::ValF32(f as f32))
            },
            _ => {
                return Err("".into());
            },
        }
    }

    fn db_to_f64(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Integer(i) => {
                Ok(ParameterValue::ValF64(i as f64))
            },
            rusqlite::types::Value::Real(f) => {
                Ok(ParameterValue::ValF64(f as f64))
            },
            _ => {
                return Err("".into());
            },
        }
    }

    fn db_to_string(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Text(string) => {
                Ok(ParameterValue::ValString(string))
            },
            _ => {
                return Err("".into());
            },
        }
    }

    fn db_to_blob(sql_value: rusqlite::types::Value) -> Result<ParameterValue, Box<dyn Error>> {
        match sql_value {
            rusqlite::types::Value::Blob(blob) => {
                Ok(ParameterValue::ValBlob(blob))
            },
            _ => {
                return Err("".into());
            },
        }
    }

    pub(crate) fn read_or_create(&self, id: ParameterId) -> Result<ParameterValue, Box<dyn Error>> {
        let db = DbConnection::new(&self.database_path, false, false)?;
        
        let sql = "SELECT value FROM configuration WHERE key = ?";
        let mut stmt = match db.conn().prepare(sql) {
            Ok(s) => s,
            Err(e) => {
                return Err(format!("Failed to prepare statement: {}", e).into());
            }
        };

        let parameter_def = &PARAMETER_DATA[id as usize];
        let key = parameter_def.name_id;
        let result = match stmt.query_row(params![key], |row| {
            // Automatically detect SQLite's storage type
            let sql_value: rusqlite::types::Value = row.get(0)?;

            let value_result = match parameter_def.value {
                ParameterValue::ValBool(_) => Self::db_to_bool(sql_value),
                ParameterValue::ValI32(_) => Self::db_to_i32(sql_value),
                ParameterValue::ValU32(_) => Self::db_to_u32(sql_value),
                ParameterValue::ValI64(_) => Self::db_to_i64(sql_value),
                ParameterValue::ValU64(_) => Self::db_to_u64(sql_value),
                ParameterValue::ValF32(_) => Self::db_to_f32(sql_value),
                ParameterValue::ValF64(_) => Self::db_to_f64(sql_value),
                ParameterValue::ValString(_) =>Self::db_to_string(sql_value),
                ParameterValue::ValBlob(_) => Self::db_to_blob(sql_value),
            };
            
            match value_result {
                Ok(value) => Ok(value),
                Err(_) => {
                    println!("Type mismatch for [{}], using default", key);
                    Ok(parameter_def.value.clone())
                }
            }
        }) {
            Ok(val) => Ok(val),
            Err(_) => {
                Ok(parameter_def.value.clone())
            }
        };
        result
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
            let current = self.read_or_create(id).unwrap();
            if current == value {
                return Ok(Status::StatusOkNotChanged(value));
            }
        }
        
        let db = DbConnection::new(&self.database_path, true, false)?;
        
        let sql = "INSERT OR REPLACE INTO configuration (key, value, timestamp) VALUES (?,?,?);";
        
        let mut stmt = db.conn.as_ref().unwrap().prepare(sql)?;
        
        // Bind parameters
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
                ParameterValue::ValString(v) => v.as_str().to_sql()?,
                ParameterValue::ValBlob(v) => v.to_sql()?,
                _ => 0.to_sql()?,
            },
            Self::get_timestamp(),
        ])?;
        
        Ok(Status::StatusOkChanged(value))
    }
    
    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        let sql = "SELECT key FROM configuration WHERE timestamp >= ?";
        let check_start = Self::get_timestamp();
        let mut pending_callbacks: Vec<ParameterId> = Vec::new();

        let db = DbConnection::new(&self.database_path, false, false)?;

        let conn = db.conn.as_ref().ok_or("Database not open")?;
        let mut stmt = conn.prepare(sql)?;
        let mut rows = stmt.query(params![self.last_update_timestamp])?;

        while let Some(row) = rows.next()? {
            let key = row.get::<usize, String>(0)?;

            let id = PARAMETER_DATA.iter()
                        .position(|pm| pm.name_id == key)
                        .expect("Parameter not found");

            let pm_id = match ParameterId::try_from(id) {
                Ok(param) => {
                    param
                }
                Err(_) => {
                    return Err(format!("Invalid parameter value: {}", id).into());
                }
            };

            // let parameter_def = &PARAMETER_DATA[id as usize];
            // let sql_value = row.get(1)?;
            // let value_result = match parameter_def.value {
            //     ParameterValue::ValBool(_) => Self::db_to_bool(sql_value),
            //     ParameterValue::ValI32(_) => Self::db_to_i32(sql_value),
            //     ParameterValue::ValU32(_) => Self::db_to_u32(sql_value),
            //     ParameterValue::ValI64(_) => Self::db_to_i64(sql_value),
            //     ParameterValue::ValU64(_) => Self::db_to_u64(sql_value),
            //     ParameterValue::ValF32(_) => Self::db_to_f32(sql_value),
            //     ParameterValue::ValF64(_) => Self::db_to_f64(sql_value),
            //     ParameterValue::ValString(_) =>Self::db_to_string(sql_value),
            //     ParameterValue::ValBlob(_) => Self::db_to_blob(sql_value),
            // };

            // validate

            pending_callbacks.push(pm_id);
        }

        self.last_update_timestamp = check_start;

        for key in pending_callbacks {
            // if let Some((callback, _)) = self.callbacks.get(key) {
            //     callback();
            // }
        }

        Ok(())
    }

}
