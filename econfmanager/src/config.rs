pub(crate) struct Config {
    pub database_path: String,
    pub saved_database_path: String,
    pub proto_name: String,
}

impl Config {
    pub(crate) fn new(proto_name: &String, database_path: &String, saved_database_path: &String) -> Result<Config, Box<dyn std::error::Error>> {
        Ok(Config{proto_name: proto_name.to_string(), database_path: database_path.to_string(), saved_database_path: saved_database_path.to_string() })
    }
}
