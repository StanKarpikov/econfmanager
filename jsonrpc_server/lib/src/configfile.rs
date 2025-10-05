use std::fs;
use serde::Deserialize;

/******************************************************************************
 * PUBLIC TYPES
 ******************************************************************************/

#[derive(Deserialize, Default)]
pub struct Config {
    #[serde(default = "default_database_path")]
    pub database_path: String,
    #[serde(default = "default_saved_database_path")]
    pub saved_database_path: String,
    #[serde(default = "default_default_data_folder")]
    pub default_data_folder: String,
    #[serde(default = "default_json_rpc_listen_address")]
    pub json_rpc_listen_address: String,
    #[serde(default = "default_json_rpc_port")]
    pub json_rpc_port: String,
}

#[derive(Deserialize)]
struct YamlConfig {
    econfmanager: Config,
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

fn default_default_data_folder() -> String {
    ".".to_string()
}

fn default_json_rpc_listen_address() -> String {
    "127.0.0.1".to_string()
}

fn default_json_rpc_port() -> String {
    "3030".to_string()
}

/******************************************************************************
 * PUBLIC FUNCTIONS
 ******************************************************************************/

impl Config {
    pub fn from_file(config_file: String) -> Config {
        let file_content = fs::read_to_string(std::path::Path::new(&config_file))
            .unwrap_or_else(|_| panic!("Failed to read configuration file {}", config_file));

        let config: YamlConfig = serde_yaml::from_str(&file_content)
            .expect("Failed to parse configuration");

        config.econfmanager
    }
}
