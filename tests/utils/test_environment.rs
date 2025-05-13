use mongodb::{Client, bson::Document, options::ClientOptions};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

// Test environment configuration
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
            mongodb_port: 27018, // Using a different port to avoid conflicts
            mongodb_data_dir: "./mongodb_test_data".to_string(),
            mongodb_log_path: "./mongodb_test.log".to_string(),
            app_port: 8081, // Using a different port to avoid conflicts
            database_name: "test".to_string(),
        }
    }
}

pub struct TestEnvironment {
    mongodb_process: Child,
    app_process: Child,
    pub mongodb_client: Client,
    pub config: TestConfig,
}

// Create a static Tokio runtime for database operations
lazy_static::lazy_static! {
    static ref TOKIO_RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
}

impl TestEnvironment {
    pub fn new() -> Self {
        Self::with_config(TestConfig::default())
    }

    pub fn with_config(config: TestConfig) -> Self {
        // Create data directory if it doesn't exist
        if !Path::new(&config.mongodb_data_dir).exists() {
            fs::create_dir_all(&config.mongodb_data_dir)
                .expect("Failed to create MongoDB data directory");
        }

        // Start MongoDB
        println!("Starting MongoDB...");
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
        println!("MongoDB started successfully");

        // Start our application with environment variables
        println!("Starting application server...");
        let mongodb_uri = format!("mongodb://localhost:{}", config.mongodb_port);

        // Use Command::new with environment variables instead of unsafe set_var
        let app_process = Command::new("cargo")
            .args(["run", "--", "--port", &config.app_port.to_string()])
            .env("DATABASE_CONN_URL", &mongodb_uri)
            .env("DATABASE_NAME", &config.database_name)
            .spawn()
            .expect("Failed to start application server");

        // Wait for our application to start
        wait_for_tcp_port(config.app_port);
        println!("Application server started successfully");

        // Initialize MongoDB client using the static runtime
        let mongodb_client = TOKIO_RUNTIME.block_on(async {
            let client_options = ClientOptions::parse(&mongodb_uri)
                .await
                .expect("Failed to parse MongoDB connection string");

            Client::with_options(client_options).expect("Failed to connect to MongoDB")
        });

        TestEnvironment {
            mongodb_process,
            app_process,
            mongodb_client,
            config,
        }
    }

    pub fn insert_test_data(&self, collection_name: &str, documents: Vec<Document>) {
        // Insert data into the collection (without dropping the database)
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

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        // Stop our application
        println!("Stopping application server...");
        self.app_process
            .kill()
            .expect("Failed to stop application server");

        // Stop MongoDB
        println!("Stopping MongoDB...");
        self.mongodb_process.kill().expect("Failed to stop MongoDB");

        // Clean up MongoDB data directory
        println!("Cleaning up test data...");

        // Wait a moment to ensure MongoDB has fully released the files
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Remove data directory if it exists
        if Path::new(&self.config.mongodb_data_dir).exists() {
            // Try multiple times with a delay between attempts
            for attempt in 1..=5 {
                match fs::remove_dir_all(&self.config.mongodb_data_dir) {
                    Ok(_) => {
                        println!("Successfully removed MongoDB data directory");
                        break;
                    }
                    Err(e) => {
                        if attempt == 5 {
                            println!("Warning: Failed to remove MongoDB data directory: {}", e);
                            // Don't panic, just log the error
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
        if Path::new(&self.config.mongodb_log_path).exists() {
            match fs::remove_file(&self.config.mongodb_log_path) {
                Ok(_) => println!("Successfully removed MongoDB log file"),
                Err(e) => println!("Warning: Failed to remove MongoDB log file: {}", e),
                // Don't panic, just log the error
            }
        }
    }
}

pub fn wait_for_tcp_port(port: u16) {
    let addr = format!("127.0.0.1:{}", port);
    let max_attempts = 30;
    let mut attempts = 0;

    while attempts < max_attempts {
        match TcpStream::connect(&addr) {
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
    let addr = format!("127.0.0.1:{}", port);
    let mut stream = TcpStream::connect(addr).expect("Failed to connect to application server");

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        path
    );
    stream
        .write_all(request.as_bytes())
        .expect("Failed to send HTTP request");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("Failed to read HTTP response");

    // Parse status code
    let status_line = response.lines().next().unwrap_or("");
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("500")
        .parse::<u16>()
        .unwrap_or(500);

    // Extract response body (after the double CRLF)
    let body = response.split("\r\n\r\n").nth(1).unwrap_or("").to_string();

    (status_code, body)
}
