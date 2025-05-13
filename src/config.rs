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

    // Use sensible defaults if environment variables are not set
    let database_name = env::var("DATABASE_NAME").unwrap_or_else(|_| "test".to_string());
    let database_username = env::var("DATABASE_USERNAME").unwrap_or_else(|_| "".to_string());
    let database_password = env::var("DATABASE_PASSWORD").unwrap_or_else(|_| "".to_string());

    // Build connection URL with or without credentials
    let database_conn_url = env::var("DATABASE_CONN_URL").unwrap_or_else(|_| {
        if database_username.is_empty() || database_password.is_empty() {
            "mongodb://localhost:27017".to_string()
        } else {
            format!(
                "mongodb://{}:{}@localhost:27017",
                database_username, database_password
            )
        }
    });

    AppConfig {
        database_name,
        database_username,
        database_password,
        database_conn_url,
    }
}
