use mongodb::{Client, bson::Document, options::ClientOptions};
use reqwest::blocking::Client as ReqwestClient;
use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::{Mutex, Once};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

// Test environment configuration
#[derive(Clone)]
pub struct TestConfig {
    pub mongodb_port: u16,
    pub mongodb_data_dir: String,
    pub mongodb_log_path: String,
    pub app_port: u16,
    pub database_name: String,
}

impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            mongodb_port: 27018, // Using a non-default port to avoid conflicts
            mongodb_data_dir: "./test-dbpath/mongodb_test_data".to_string(),
            mongodb_log_path: "./test-dbpath/mongodb_test.log".to_string(),
            app_port: 8081, // Using a non-default port to avoid conflicts
            database_name: "test".to_string(),
        }
    }
}

// Create a static Tokio runtime for database operations
lazy_static::lazy_static! {
    static ref TOKIO_RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
    static ref SHARED_MONGODB: Mutex<Option<SharedMongoDB>> = Mutex::new(None);
    static ref INIT_MONGODB: Once = Once::new();
    static ref CLEANUP_MONGODB: Once = Once::new();
    static ref SETUP_DONE: Once = Once::new();
}

// Struct to hold the MongoDB process and client
struct SharedMongoDB {
    process: Child,
    client: Client,
    config: TestConfig,
}

// Shared test environment that reuses the MongoDB instance
pub struct SharedTestEnvironment {
    app_process: Child,
    pub mongodb_client: Client,
    pub config: TestConfig,
}

impl SharedTestEnvironment {
    pub fn new() -> Self {
        Self::with_config(TestConfig::default())
    }

    pub fn with_config(config: TestConfig) -> Self {
        // Initialize MongoDB if it's not already running
        initialize_mongodb(&config);

        // Get a reference to the MongoDB client
        let mongodb_client = get_mongodb_client();

        // Start our application with environment variables
        println!("Starting application server...");
        let mongodb_uri = format!("mongodb://localhost:{}", config.mongodb_port);

        // Use Command::new with environment variables
        let app_process = Command::new("cargo")
            .args(["run", "--", "--port", &config.app_port.to_string()])
            .env("DATABASE_CONN_URL", &mongodb_uri)
            .env("DATABASE_NAME", &config.database_name)
            .spawn()
            .expect("Failed to start application server");

        // Wait for our application to start
        wait_for_tcp_port(config.app_port);
        println!("Application server started successfully");

        SharedTestEnvironment {
            app_process,
            mongodb_client,
            config,
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
}

impl Drop for SharedTestEnvironment {
    fn drop(&mut self) {
        // Stop our application
        println!("Stopping application server...");
        self.app_process
            .kill()
            .expect("Failed to stop application server");

        // Schedule cleanup to run once when all tests are done
        CLEANUP_MONGODB.call_once(|| {
            cleanup_mongodb();
        });
    }
}

// Initialize MongoDB if it's not already running
fn initialize_mongodb(config: &TestConfig) {
    INIT_MONGODB.call_once(|| {
        // Create data directory if it doesn't exist
        if !Path::new(&config.mongodb_data_dir).exists() {
            fs::create_dir_all(&config.mongodb_data_dir)
                .expect("Failed to create MongoDB data directory");
        }

        // Start MongoDB
        println!("Starting shared MongoDB instance...");
        let mongodb_process = Command::new("mongod")
            .args([
                "--port",
                &config.mongodb_port.to_string(),
                "--dbpath",
                &config.mongodb_data_dir,
                "--logpath",
                &config.mongodb_log_path,
                "--fork", // Run in background
                "--bind_ip",
                "127.0.0.1",
            ])
            .spawn()
            .expect("Failed to start MongoDB");

        // Wait for MongoDB to start
        wait_for_tcp_port(config.mongodb_port);
        println!("Shared MongoDB instance started successfully");

        // Initialize MongoDB client
        let mongodb_uri = format!("mongodb://localhost:{}", config.mongodb_port);
        let mongodb_client = TOKIO_RUNTIME.block_on(async {
            let client_options = ClientOptions::parse(&mongodb_uri)
                .await
                .expect("Failed to parse MongoDB connection string");

            Client::with_options(client_options).expect("Failed to connect to MongoDB")
        });

        // Store the MongoDB process and client
        let mut shared_mongodb = SHARED_MONGODB.lock().unwrap();
        *shared_mongodb = Some(SharedMongoDB {
            process: mongodb_process,
            client: mongodb_client.clone(),
            config: config.clone(),
        });

        // Run database setup (drop the database to start with a clean state)
        SETUP_DONE.call_once(|| {
            TOKIO_RUNTIME.block_on(async {
                println!(
                    "Dropping database {} for initial setup",
                    config.database_name
                );
                mongodb_client
                    .database(&config.database_name)
                    .drop()
                    .await
                    .expect("Failed to drop database");
                println!("Database {} dropped successfully", config.database_name);
            });
            println!("Database setup complete - dropped database before tests");
        });
    });
}

// Get a reference to the MongoDB client
fn get_mongodb_client() -> Client {
    let shared_mongodb = SHARED_MONGODB.lock().unwrap();
    match &*shared_mongodb {
        Some(mongodb) => mongodb.client.clone(),
        None => panic!("MongoDB not initialized"),
    }
}

// Clean up MongoDB
fn cleanup_mongodb() {
    let mut shared_mongodb = SHARED_MONGODB.lock().unwrap();
    if let Some(mut mongodb) = shared_mongodb.take() {
        // Stop MongoDB
        println!("Stopping shared MongoDB instance...");
        mongodb.process.kill().expect("Failed to stop MongoDB");

        // Clean up MongoDB data directory
        println!("Cleaning up test data...");

        // Wait a moment to ensure MongoDB has fully released the files
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Remove data directory if it exists
        if Path::new(&mongodb.config.mongodb_data_dir).exists() {
            // Try multiple times with a delay between attempts
            for attempt in 1..=5 {
                match fs::remove_dir_all(&mongodb.config.mongodb_data_dir) {
                    Ok(_) => {
                        println!("Successfully removed MongoDB data directory");
                        break;
                    }
                    Err(e) => {
                        if attempt == 5 {
                            println!("Warning: Failed to remove MongoDB data directory: {}", e);
                        } else {
                            println!(
                                "Attempt {} to remove data directory failed, retrying...",
                                attempt
                            );
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                    }
                }
            }
        }

        // Remove log file if it exists
        if Path::new(&mongodb.config.mongodb_log_path).exists() {
            match fs::remove_file(&mongodb.config.mongodb_log_path) {
                Ok(_) => println!("Successfully removed MongoDB log file"),
                Err(e) => println!("Warning: Failed to remove MongoDB log file: {}", e),
            }
        }
    }
}

pub fn wait_for_tcp_port(port: u16) {
    let url = format!("http://127.0.0.1:{}", port);
    let client = ReqwestClient::new();
    let max_attempts = 30;
    let mut attempts = 0;

    while attempts < max_attempts {
        match client.get(&url).timeout(Duration::from_secs(1)).send() {
            Ok(_) => return,
            Err(_) => {
                attempts += 1;
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    panic!("Timed out waiting for port {} to be available", port);
}

// Make HTTP request to the test server
pub fn make_http_request(path: &str) -> (u16, String) {
    // Use the default test config
    make_http_request_with_port(path, TestConfig::default().app_port)
}

// Make HTTP request to a specific port
pub fn make_http_request_with_port(path: &str, port: u16) -> (u16, String) {
    let url = format!("http://127.0.0.1:{}{}", port, path);
    let client = ReqwestClient::new();

    let response = client
        .get(&url)
        .send()
        .expect("Failed to send HTTP request");

    let status_code = response.status().as_u16();
    let body = response.text().expect("Failed to read HTTP response");

    (status_code, body)
}
