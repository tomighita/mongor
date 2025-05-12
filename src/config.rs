use serde::Deserialize;
use std::env;

use dotenv::dotenv;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database_name: String,
    pub database_username: String,
    pub database_password: String,
    pub database_conn_url: String,
}

pub fn load_config() -> AppConfig {
    // Load environment variables from the .env file
    dotenv().ok();

    AppConfig {
        database_name: env::var("DATABASE_NAME")
            .expect("SERVER_HOST not set")
            .to_string(),
        database_username: env::var("DATABASE_USERNAME")
            .expect("DATABASE_USERNAME not set")
            .to_string(),
        database_password: env::var("DATABASE_PASSWORD")
            .expect("DATABASE_PASSWORD not set")
            .to_string(),
        database_conn_url: env::var("DATABASE_CONN_URL")
            .expect("DATABASE_CONN_URL not set")
            .to_string(),
    }
}
