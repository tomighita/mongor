use mongodb::Client;
use std::sync::Once;
use tokio::runtime::Runtime;

// Create a static Tokio runtime for database operations
lazy_static::lazy_static! {
    static ref TOKIO_RUNTIME: Runtime = Runtime::new().expect("Failed to create Tokio runtime");
}

// Used to ensure setup runs only once before all tests
static SETUP: Once = Once::new();
static TEARDOWN: Once = Once::new();

// Setup function to run before all tests
pub fn setup(client: &Client, database_name: &str) {
    SETUP.call_once(|| {
        // Drop the database
        TOKIO_RUNTIME.block_on(async {
            println!("Dropping database {}", database_name);
            client
                .database(database_name)
                .drop()
                .await
                .expect("Failed to drop database");
            println!("Database {} dropped successfully", database_name);
        });
        println!("Database setup complete - dropped database before tests");
    });
}

// Teardown function to run after all tests
pub fn teardown(client: &Client, database_name: &str) {
    TEARDOWN.call_once(|| {
        // Drop the database
        TOKIO_RUNTIME.block_on(async {
            println!("Dropping database {}", database_name);
            client
                .database(database_name)
                .drop()
                .await
                .expect("Failed to drop database");
            println!("Database {} dropped successfully", database_name);
        });
        println!("Database teardown complete - dropped database after tests");
    });
}
