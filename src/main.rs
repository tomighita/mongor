use dotenv::dotenv;
use mongodb::{Client, bson::doc, options::ClientOptions};
use serde::Deserialize;
use std::env;
use tokio;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub database_name: String,
    pub database_username: String,
    pub database_password: String,
    pub database_conn_url: String,
}

fn load_config() -> AppConfig {
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

#[tokio::main]
async fn main() -> mongodb::error::Result<()> {
    let config = load_config();

    // Configure the MongoDB client
    let options = ClientOptions::parse(config.database_conn_url).await?;
    let client = Client::with_options(options)?;

    // Access a database and collection
    let database = client.database(&config.database_name);

    // Ping the database
    let resp = database.run_command(doc! { "ping": 1 }).await?;
    println!(
        "Pinged your deployment. You successfully connected to MongoDB!: {}",
        resp.to_string()
    );
    Ok(())
}
