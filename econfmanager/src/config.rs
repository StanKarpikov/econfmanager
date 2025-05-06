pub(crate) struct Config {
    pub database_path: String,
    pub saved_database_path: String,
    pub default_data_folder: String,
}

impl Config {
    pub(crate) fn new(database_path: &String, saved_database_path: &String, default_data_folder: &String) -> Result<Config, Box<dyn std::error::Error>> {
        Ok(Config{database_path: database_path.to_string(), saved_database_path: saved_database_path.to_string(), default_data_folder: default_data_folder.to_string() })
    }
}
