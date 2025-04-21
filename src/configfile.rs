
use std::fs;
use serde::Deserialize;

#[path = "arguments.rs"] pub mod arguments;
use arguments::Args;

#[derive(Deserialize, Default)]
pub(crate) struct Config {
    #[serde(default = "default_descriptors_path")]
    pub descriptors_path: String,
    #[serde(default = "default_proto_name")]
    pub proto_name: String,
    #[serde(default = "default_database_path")]
    pub database_path: String,
}

fn default_descriptors_path() -> String {
    "descriptors.bin".to_string()
}

fn default_proto_name() -> String {
    "configuration.proto".to_string()
}

fn default_database_path() -> String {
    "configuration.db".to_string()
}

pub(crate) fn parse_config_file(args: Args) -> Config {
    let file_content = fs::read_to_string(std::path::Path::new(&args.config)).expect("Failed to read the file");
    let config: Config = serde_json::from_str(&file_content).expect("Failed to parse JSON");
    config
}