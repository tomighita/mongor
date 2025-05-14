use mongodb::{Client, options::ClientOptions};
use reqwest::blocking::Client as ReqwestClient;
use std::fs;
use std::path::Path;
use std::process::Child;
use std::thread;
use std::time::Duration;

use crate::utils::test_environment::{TOKIO_RUNTIME, TestConfig};

// Start MongoDB and return the process
pub fn start_mongodb(config: &TestConfig) -> Child {
    // Create data directory if it doesn't exist
    if !Path::new(&config.mongodb_data_dir).exists() {
        fs::create_dir_all(&config.mongodb_data_dir)
            .expect("Failed to create MongoDB data directory");
    }

    // Start MongoDB
    println!("Starting MongoDB instance...");
    let mongodb_process = std::process::Command::new("mongod")
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
    println!("MongoDB instance started successfully");

    // Initialize a temporary MongoDB client to drop the database
    let mongodb_uri = format!("mongodb://localhost:{}", config.mongodb_port);
    let mongodb_client = TOKIO_RUNTIME.block_on(async {
        let client_options = ClientOptions::parse(&mongodb_uri)
            .await
            .expect("Failed to parse MongoDB connection string");

        Client::with_options(client_options).expect("Failed to connect to MongoDB")
    });

    // Run database setup (drop the database to start with a clean state)
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

    mongodb_process
}

// Clean up MongoDB resources
pub fn cleanup_mongodb(process: &mut Child, config: &TestConfig) {
    println!("Stopping MongoDB instance...");
    process.kill().expect("Failed to stop MongoDB");

    // Wait for files to be released
    thread::sleep(Duration::from_millis(500));

    // Clean up data directory
    if Path::new(&config.mongodb_data_dir).exists() {
        match fs::remove_dir_all(&config.mongodb_data_dir) {
            Ok(_) => println!("Successfully removed MongoDB data directory"),
            Err(e) => println!("Warning: Failed to remove MongoDB data directory: {}", e),
        }
    }

    // Clean up log file
    if Path::new(&config.mongodb_log_path).exists() {
        match fs::remove_file(&config.mongodb_log_path) {
            Ok(_) => println!("Successfully removed MongoDB log file"),
            Err(e) => println!("Warning: Failed to remove MongoDB log file: {}", e),
        }
    }
}

// Wait for a TCP port to be available
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
