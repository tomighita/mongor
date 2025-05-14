use mongodb::bson::{Document, doc};
use serde_json;
use serial_test::serial;

// Import test environment and utilities from utils module
mod utils;
use utils::test_environment::TestEnvironment;
use utils::utils::make_http_request;

// Base name for test collections
static TEST_COLLECTION_BASE_NAME: &str = "mongor_get_endpoint_test";

// Generate a unique collection name for each test
fn unique_collection_name(test_name: &str) -> String {
    format!(
        "{}_{}",
        TEST_COLLECTION_BASE_NAME,
        test_name.replace(" ", "_")
    )
}

/// Run a query endpoint test with the given parameters
///
/// This function:
/// 1. Creates a test environment (MongoDB + app)
/// 2. Creates a collection with a unique name and inserts test documents
/// 3. Makes a request to the endpoint
/// 4. Verifies the response matches the expected documents
/// 5. Automatically cleans up all processes when the test environment is dropped
fn run_get_endpoint_test(
    test_name: &str,
    request_path: &str,
    test_documents: Vec<Document>,
    expected_documents: Vec<Document>,
) {
    // Create test environment (starts MongoDB and app)
    let env = TestEnvironment::new();

    // Generate a unique collection name for this test
    let collection_name = unique_collection_name(test_name);

    // Insert test data (the insert_test_data method will drop the collection first)
    env.insert_test_data(&collection_name, test_documents);

    // Make a request to our endpoint
    let full_request_path = format!("/{}{}", collection_name, request_path);
    let (status_code, body) = make_http_request(&full_request_path);

    // Verify the response
    assert_eq!(
        status_code, 200,
        "Expected status code 200, got {}",
        status_code
    );

    // Parse the JSON response into BSON documents
    let documents: Vec<Document> =
        serde_json::from_str(&body).expect("Failed to parse JSON response");

    // Verify the response matches expected documents directly
    assert_eq!(
        documents, expected_documents,
        "Documents don't match expected values"
    );

    // The test environment will be automatically cleaned up when it goes out of scope
}

#[test]
#[serial]
fn test_get_with_matching_document() {
    // Test document
    let test_doc = doc! {
        "_id": 1,
        "name": "test document"
    };

    // Run the test
    run_get_endpoint_test(
        "matching document",
        "?match:_id:1",
        vec![test_doc.clone()],
        vec![test_doc],
    );
}

#[test]
#[serial]
fn test_get_with_non_existent_document() {
    // Test document (to ensure collection exists)
    let test_doc = doc! {
        "_id": 1,
        "name": "test document"
    };

    // Run the test
    run_get_endpoint_test(
        "non-existent document",
        "?match:_id:999",
        vec![test_doc],
        Vec::new(), // Expect empty array
    );
}

#[test]
#[serial]
fn test_get_with_multiple_documents() {
    // Test documents
    let docs = vec![
        doc! {
            "_id": 1,
            "name": "first document",
            "category": "A"
        },
        doc! {
            "_id": 2,
            "name": "second document",
            "category": "A"
        },
        doc! {
            "_id": 3,
            "name": "third document",
            "category": "B"
        },
    ];

    // Run the test for category A
    run_get_endpoint_test(
        "multiple documents",
        "?match:category:A",
        docs.clone(),
        vec![docs[0].clone(), docs[1].clone()], // Expect first two documents
    );
}
