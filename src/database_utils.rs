use std::{error::Error, time::{SystemTime, UNIX_EPOCH}};
use prost_reflect::{DynamicMessage, ReflectMessage, Value};
use rusqlite::{Connection, Transaction};

use crate::configfile::Config;

pub(crate) struct DatabaseManager {
    conn: Connection
}

impl DatabaseManager {

    /******************************************************************************
     * PRIVATE FUNCTIONS
     ******************************************************************************/

    fn check_create_database(&self) -> Result<(), Box<dyn Error>> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS configuration (
                key TEXT unique PRIMARY KEY,
                value ANY,
                timestamp REAL
            ) WITHOUT ROWID;",
            [],
        )?;

        if let Err(e) = self.configure_sqlite() {
            eprintln!("Failed to configure SQLite: {}", e);
            return Err(e.into());
        }

        Ok(())
    }
    
    fn configure_sqlite(&self) -> Result<(), Box<dyn Error>> {
        self.conn.pragma_update(None, "locking_mode", "NORMAL")?;
        self.conn.pragma_update(None, "journal_mode", "WAL")?;
    
        // TODO: Optional: needs testing
        self.conn.pragma_update(None, "wal_autocheckpoint", "1000")?;  // Pages
        self.conn.pragma_update(None, "synchronous", "NORMAL")?;
        self.conn.pragma_update(None, "busy_timeout", "10000")?;  // 10 second timeout
        Ok(())  
    }
    
    fn insert_fields(
        tx: &Transaction,
        message: &DynamicMessage,
        prefix: &str,
        timestamp: f64
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stmt = tx.prepare("INSERT OR REPLACE INTO configuration (key, value, timestamp) VALUES (?1, ?2, ?3)")?;
    
        for field in message.descriptor().fields() {
            let field_name = field.name();
            let full_key = if prefix.is_empty() {
                field_name.to_string()
            } else {
                format!("{}@{}", prefix, field_name)
            };
            
            let value = &*message.get_field(&field);
            let sql_value = match value {
                Value::I32(v) => rusqlite::types::Value::Integer(*v as i64),
                Value::I64(v) => rusqlite::types::Value::Integer(*v),
                Value::U32(v) => rusqlite::types::Value::Integer(*v as i64),
                Value::U64(v) => rusqlite::types::Value::Integer(*v as i64),
                Value::F32(v) => rusqlite::types::Value::Real(*v as f64),
                Value::F64(v) => rusqlite::types::Value::Real(*v),
                Value::Bool(v) => rusqlite::types::Value::Integer(if *v { 1 } else { 0 }),
                Value::String(v) => rusqlite::types::Value::Text(v.clone()),
                Value::Bytes(v) => rusqlite::types::Value::Blob(v.to_vec()),
                _ => rusqlite::types::Value::Null,
            };
            
            stmt.execute((&full_key, sql_value, timestamp))?;
        }
        Ok(())
    }
    
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
        let conn = Connection::open(std::path::Path::new(&config.database_path))?;
    
        let database_manager = Self { conn };

        database_manager.check_create_database()?;
        
        Ok(database_manager)
    }

    pub(crate) fn set_sqlite_version(&self, version: u32) -> Result<(), Box<dyn Error>> {
        self.conn.pragma_update(None, "user_version", version)?;
    
        // let user_version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        // println!("Database user_version set to: {}", user_version);
    
        Ok(())
    }
    
    pub(crate) fn process_config(
        &mut self,
        message: &DynamicMessage,
        prefix: &str
    ) -> Result<(), Box<dyn std::error::Error>> {
        let timestamp = Self::get_timestamp();
        let tx = self.conn.transaction()?;
        {
            let _stmt = tx.prepare("INSERT OR REPLACE INTO configuration (key, value, timestamp) VALUES (?1, ?2, ?3)")?;
    
            for field in message.descriptor().fields() {
                let field_name = field.name();
                let full_key = if prefix.is_empty() {
                    field_name.to_string()
                } else {
                    format!("{}@{}", prefix, field_name)
                };
                
                let value = &*message.get_field(&field);
                match value {
                    Value::Message(nested_msg) => {
                        Self::insert_fields(
                            &tx,
                            &nested_msg,
                            &full_key,
                            timestamp,
                        )?;
                    }
                    _ => {
                        return Err(format!("Field {} will be ignored, the configuration requires two levels of definitions", full_key).into());
                    }
                }
            }
        }
        tx.commit()?;
        Ok(())
    }
}
