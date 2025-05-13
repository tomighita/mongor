use mongodb::{Client, bson::Document};
use std::process::{Child, Command};
use std::sync::mpsc;
use std::thread;
use tokio::runtime::Runtime;

use crate::utils::utils;

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
    pub static ref TOKIO_RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
    static ref MONGODB_ACTOR: MongoDBActorHandle = MongoDBActorHandle::new();
}

// Simple actor for MongoDB management
struct MongoDBActorHandle {
    sender: mpsc::Sender<MongoDBActorMessage>,
}

// Messages for the MongoDB actor
enum MongoDBActorMessage {
    GetClient(mpsc::Sender<(Client, TestConfig)>),
    Shutdown,
}

impl MongoDBActorHandle {
    fn new() -> Self {
        let (sender, receiver) = mpsc::channel();

        // Spawn the actor in a separate thread
        thread::spawn(move || {
            let config = TestConfig::default();
            let mut process: Option<Child> = None;
            let mut client: Option<Client> = None;

            // Actor message loop
            while let Ok(msg) = receiver.recv() {
                match msg {
                    MongoDBActorMessage::GetClient(response) => {
                        // Initialize MongoDB if needed
                        if client.is_none() {
                            let (mongodb_process, mongodb_client) =
                                utils::initialize_mongodb(&config);
                            process = Some(mongodb_process);
                            client = Some(mongodb_client);
                        }

                        // Send the client and config back
                        response
                            .send((client.as_ref().unwrap().clone(), config.clone()))
                            .expect("Failed to send client and config");
                    }
                    MongoDBActorMessage::Shutdown => {
                        // Clean up MongoDB
                        if let Some(mut p) = process.take() {
                            utils::cleanup_mongodb(&mut p, &config);
                        }
                        break; // Exit the actor loop
                    }
                }
            }
        });

        MongoDBActorHandle { sender }
    }

    fn get_client(&self) -> (Client, TestConfig) {
        // Note: We need to create a new channel here because we need to receive the Client
        // from the actor. In shutdown(), we don't need to receive anything back.
        let (sender, receiver) = mpsc::channel();
        self.sender
            .send(MongoDBActorMessage::GetClient(sender))
            .expect("MongoDB actor has died");
        receiver
            .recv()
            .expect("Failed to receive MongoDB client and config")
    }

    fn shutdown(&self) {
        self.sender
            .send(MongoDBActorMessage::Shutdown)
            .expect("MongoDB actor has died");
    }
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
        // Get a client and config from the MongoDB actor (this will initialize MongoDB if needed)
        let (mongodb_client, actor_config) = MONGODB_ACTOR.get_client();

        // Start our application with environment variables
        println!("Starting application server...");
        let mongodb_uri = format!("mongodb://localhost:{}", actor_config.mongodb_port);

        // Use Command::new with environment variables
        let app_process = Command::new("cargo")
            .args(["run", "--", "--port", &config.app_port.to_string()])
            .env("DATABASE_CONN_URL", &mongodb_uri)
            .env("DATABASE_NAME", &actor_config.database_name)
            .spawn()
            .expect("Failed to start application server");

        // Wait for our application to start
        utils::wait_for_tcp_port(config.app_port);
        println!("Application server started successfully");

        // Use the actor's config for MongoDB-related settings, but keep the app_port from the provided config
        let combined_config = TestConfig {
            mongodb_port: actor_config.mongodb_port,
            mongodb_data_dir: actor_config.mongodb_data_dir,
            mongodb_log_path: actor_config.mongodb_log_path,
            database_name: actor_config.database_name,
            app_port: config.app_port,
        };

        SharedTestEnvironment {
            app_process,
            mongodb_client,
            config: combined_config,
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

        // Shutdown MongoDB when the last test environment is dropped
        // The actor model ensures this happens only once
        MONGODB_ACTOR.shutdown();
    }
}
