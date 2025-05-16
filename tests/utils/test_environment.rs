use mongodb::{Client, bson::Document, options::ClientOptions};
use std::process::Child;
use tokio::runtime::Runtime;

use crate::utils::utils;

// Create a static Tokio runtime for database operations
lazy_static::lazy_static! {
    pub static ref TOKIO_RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
}

// Test environment configuration
#[derive(Clone)]
pub struct TestConfig {
    pub mongodb_port: u16,
    pub mongodb_data_dir: String,
    pub mongodb_log_path: String,
    pub app_port: u16,
    pub database_name: String,
}

// Use fixed ports for tests since they run serially
static MONGODB_PORT: u16 = 27018;
static APP_PORT: u16 = 8081;

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            mongodb_port: MONGODB_PORT,
            mongodb_data_dir: format!("./test-dbpath/mongodb_test_data_{}", MONGODB_PORT),
            mongodb_log_path: format!("./test-dbpath/mongodb_test_{}.log", MONGODB_PORT),
            app_port: APP_PORT,
            database_name: "test".to_string(),
        }
    }
}

// Test environment that manages MongoDB and app processes
pub struct TestEnvironment {
    pub mongodb_client: Client,
    pub config: TestConfig,
    mongodb_process: Child,
    app_process: Child,
}

impl TestEnvironment {
    pub fn new() -> Self {
        Self::with_config(TestConfig::default())
    }

    pub fn with_config(config: TestConfig) -> Self {
        // Start MongoDB
        println!("Starting MongoDB instance...");
        let mongodb_process = utils::start_mongodb(&config);

        // Create a MongoDB client
        let mongodb_uri = format!("mongodb://localhost:{}", config.mongodb_port);
        let mongodb_client = TOKIO_RUNTIME.block_on(async {
            let client_options = ClientOptions::parse(&mongodb_uri)
                .await
                .expect("Failed to parse MongoDB connection string");

            Client::with_options(client_options).expect("Failed to connect to MongoDB")
        });

        // Start the application
        println!("Starting application server...");
        let app_process = std::process::Command::new("cargo")
            .args(["run", "--", "--port", &config.app_port.to_string()])
            .env("DATABASE_CONN_URL", &mongodb_uri)
            .env("DATABASE_NAME", &config.database_name)
            .spawn()
            .expect("Failed to start application server");

        // Wait for the application to start
        utils::wait_for_tcp_port(config.app_port);
        println!("Application server started successfully");

        TestEnvironment {
            mongodb_client,
            config,
            mongodb_process,
            app_process,
        }
    }

    pub fn insert_test_data(&self, collection_name: &str, documents: Vec<Document>) {
        // Insert data into the collection
        TOKIO_RUNTIME.block_on(async {
            // Get a handle to the collection
            let collection = self
                .mongodb_client
                .database(&self.config.database_name)
                .collection::<Document>(collection_name);

            // Drop the collection if it exists
            collection.drop().await.ok(); // Ignore errors if collection doesn't exist
            println!("Collection {} dropped successfully", collection_name);

            // Insert the new documents
            if !documents.is_empty() {
                println!(
                    "Inserting {} documents into {}",
                    documents.len(),
                    collection_name
                );
                collection
                    .insert_many(documents)
                    .await
                    .expect("Failed to insert test data");
            }
        });

        println!("Test data inserted successfully");
    }

    // Explicitly shut down the MongoDB client
    // This should be called before the environment is dropped if possible
    pub fn shutdown_client(&self) {
        // Close the MongoDB client (using a clone)
        println!("Closing MongoDB client...");
        let client_clone = self.mongodb_client.clone();
        TOKIO_RUNTIME.block_on(async {
            client_clone.shutdown().await;
        });
    }
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Shut down the MongoDB client
        self.shutdown_client();

        // Stop the application
        println!("Stopping application server...");
        let _ = self.app_process.kill();

        // Stop MongoDB
        println!("Stopping MongoDB instance...");
        utils::cleanup_mongodb(&mut self.mongodb_process, &self.config);

        // Also try to kill any lingering processes by port
        println!("Checking for lingering processes...");
        // Kill MongoDB on port
        let mongodb_pattern = format!("mongod.*{}", self.config.mongodb_port);
        let _ = std::process::Command::new("pkill")
            .args(["-f", &mongodb_pattern])
            .output();

        // Kill app on port
        let app_pattern = format!("mongor.*{}", self.config.app_port);
        let _ = std::process::Command::new("pkill")
            .args(["-f", &app_pattern])
            .output();

        // Add a delay to ensure ports are released
        println!("Waiting for ports to be released...");
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}
