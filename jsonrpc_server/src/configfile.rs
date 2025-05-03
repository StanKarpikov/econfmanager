use std::fs;
use serde::Deserialize;

/******************************************************************************
 * PUBLIC TYPES
 ******************************************************************************/

#[derive(Deserialize, Default)]
pub(crate) struct Config {
    #[allow(unused)]
    #[serde(default = "default_proto_name")]
    pub proto_name: String,
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_saved_database_path")]
    pub saved_database_path: String,
}

/******************************************************************************
 * PRIVATE FUNCTIONS
 ******************************************************************************/

fn default_proto_name() -> String {
    "configuration.proto".to_string()
}

fn default_database_path() -> String {
    "configuration.db".to_string()
}

fn default_saved_database_path() -> String {
    "configuration_saved.db".to_string()
}

/******************************************************************************
 * PUBLIC FUNCTIONS
 ******************************************************************************/

impl Config {
    pub(crate) fn new(proto_name: &String, database_path: &String, saved_database_path: &String) -> Result<Config, Box<dyn std::error::Error>> {
        Ok(Config{proto_name: proto_name.to_string(), database_path: database_path.to_string(), saved_database_path: saved_database_path.to_string() })
    }

    #[allow(unused)]
    pub(crate) fn from_file(config_file:String) -> Config {
        let file_content = fs::read_to_string(std::path::Path::new(&config_file)).expect("Failed to read the file");
        let config: Config = serde_json::from_str(&file_content).expect("Failed to parse JSON");
        config
    }
}
