use log::info;

pub(crate) struct Config {
    pub database_path: String,
    pub saved_database_path: String,
    pub default_data_folder: String,
}

impl Config {
    pub(crate) fn new(database_path: &String, saved_database_path: &String, default_data_folder: &String) -> Result<Config, Box<dyn std::error::Error>> {
        let expand_path = |path: &String| -> Result<String, Box<dyn std::error::Error>> {
            let expanded = shellexpand::env(path)
                .map_err(|e| format!("Failed to expand environment variables: {}", e))?
                .to_string();
            Ok(expanded)
        };

        let database_path = expand_path(database_path)?;
        let saved_database_path = expand_path(saved_database_path)?;
        let default_data_folder = expand_path(default_data_folder)?;

        info!("Database path: {}", database_path);
        info!("Saved database path: {}", saved_database_path);
        info!("Default data folder: {}", default_data_folder);

        Ok(Config {
            database_path,
            saved_database_path,
            default_data_folder,
        })
    }
}
