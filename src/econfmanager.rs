use prost_reflect::{DescriptorPool, DynamicMessage, ReflectMessage, Value};
use rusqlite::{Connection, Transaction};
use std::{error::Error, time::{SystemTime, UNIX_EPOCH}};

/// Returns current timestamp with seconds and milliseconds as a floating-point number
/// (e.g. 1712345678.456 for 456 milliseconds past the second)
pub fn get_timestamp() -> f64 {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    
    let seconds = duration.as_secs() as f64;
    let milliseconds = (duration.subsec_millis() as f64) / 1000.0;
    
    seconds + milliseconds
}

fn configure_sqlite(db: &Connection) -> Result<(), Box<dyn Error>> {
    db.pragma_update(None, "locking_mode", "NORMAL")?;
    db.pragma_update(None, "journal_mode", "WAL")?;

    // TODO: Optional: needs testing
    db.pragma_update(None, "wal_autocheckpoint", "1000")?;  // Pages
    db.pragma_update(None, "synchronous", "NORMAL")?;
    db.pragma_update(None, "busy_timeout", "10000")?;  // 10 second timeout
    Ok(())
}

fn set_sqlite_version(conn: &Connection, version: u32) -> Result<(), Box<dyn Error>> {
    conn.pragma_update(None, "user_version", version)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let descriptor_path = std::path::Path::new("descriptors.bin");
    let descriptor_bytes = std::fs::read(descriptor_path)?;
    let pool = DescriptorPool::decode(&*descriptor_bytes)?;

    let config_descriptor = pool.get_message_by_name("Configuration")
        .ok_or("Configuration message not found in descriptor pool")?;
    
    let file_descriptor = pool.get_file_by_name("configuration.proto")
    .ok_or("configuration.proto file descriptor not found")?;

    let options = file_descriptor.options();

    // The field number for your extension is 60001
    let version;
    if let Some(value) = options.get_field_by_number(60001) {
        version = match &*value {
            Value::I32(v) => *v as u32,
            Value::I64(v) => *v as u32,
            Value::U32(v) => *v as u32,
            Value::U64(v) => *v as u32,
            _ => return Err("Version option is not an integer".into()),
        };
        println!("Version: {}", version);
    } else {
        return Err("Version option not found".into());
    }

    let default_config = DynamicMessage::new(config_descriptor.clone());
    
    let mut conn = Connection::open("configuration.db")?;

    set_sqlite_version(&conn, version)?;
    // let user_version: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    // println!("Database user_version set to: {}", user_version);

    if let Err(e) = configure_sqlite(&conn) {
        eprintln!("Failed to configure SQLite: {}", e);
        return Err(e.into());
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS configuration (
            key TEXT unique PRIMARY KEY,
            value ANY,
            timestamp REAL
        ) WITHOUT ROWID;",
        [],
    )?;
    
    // Get current timestamp
    let timestamp = get_timestamp();
    
    // Recursively insert all fields
    process_config(
        &mut conn,
        &default_config,
        "",
        timestamp
    )?;
    
    println!("Database created successfully!");
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

fn process_config(
    conn: &mut Connection,
    message: &DynamicMessage,
    prefix: &str,
    timestamp: f64
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.transaction()?;
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
                    insert_fields(
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