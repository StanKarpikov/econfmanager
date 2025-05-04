use std::fs;
use serde::Deserialize;

/******************************************************************************
 * PUBLIC TYPES
 ******************************************************************************/

#[derive(Deserialize, Default)]
pub(crate) struct Config {
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_saved_database_path")]
    pub saved_database_path: String,
    #[serde(default = "default_json_rpc_listen_address")]
    pub json_rpc_listen_address: String,
    #[serde(default = "default_json_rpc_port")]
    pub json_rpc_port: String,
}

/******************************************************************************
 * PRIVATE FUNCTIONS
 ******************************************************************************/

fn default_database_path() -> String {
    "configuration.db".to_string()
}

fn default_saved_database_path() -> String {
    "configuration_saved.db".to_string()
}

fn default_json_rpc_listen_address() -> String {
    "127.0.0.1".to_string()
}

fn default_json_rpc_port() -> String {
    "3000".to_string()
}

/******************************************************************************
 * PUBLIC FUNCTIONS
 ******************************************************************************/

impl Config {
    pub(crate) fn from_file(config_file:String) -> Config {
        let file_content = fs::read_to_string(std::path::Path::new(&config_file)).expect("Failed to read the file");
        let config: Config = serde_json::from_str(&file_content).expect("Failed to parse JSON");
        config
    }
}
