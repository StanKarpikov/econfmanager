use clap::Parser;
use std::error::Error;

pub mod schema;
pub mod arguments;
pub mod configfile;
pub mod database_utils;

use schema::SchemaManager;
use arguments::Args;
use configfile::{Config, parse_config_file};
use database_utils::DatabaseManager;

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let config: Config = parse_config_file(args);

    let schema = SchemaManager::new(&config)?;
    let database = DatabaseManager::new(config)?;

    schema.prepare_database(database)?;

    println!("Database created successfully!");
    Ok(())
}
