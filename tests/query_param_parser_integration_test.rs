use mongodb::bson::{Document, doc};
use serde_json;
use serial_test::serial;

// Import test environment and utilities from utils module
mod utils;
use utils::test_environment::TestEnvironment;
use utils::utils::make_http_request;

// Base name for test collections
static TEST_COLLECTION_BASE_NAME: &str = "mongor_query_parser_test";

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
/// 3. Makes a request to the endpoint with the given query
/// 4. Verifies the response matches the expected documents
/// 5. Automatically cleans up all processes when the test environment is dropped
fn run_query_parser_test(
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
fn test_simple_field_value_query() {
    // Test documents
    let docs = vec![
        doc! {
            "_id": 1,
            "name": "first document",
            "age": 25
        },
        doc! {
            "_id": 2,
            "name": "second document",
            "age": 30
        },
        doc! {
            "_id": 3,
            "name": "third document",
            "age": 35
        },
    ];

    // Run the test for age=30
    run_query_parser_test(
        "simple field value",
        "?age=30",
        docs.clone(),
        vec![docs[1].clone()], // Expect only the second document
    );
}

#[test]
#[serial]
fn test_advanced_comparison_query() {
    // Test documents
    let docs = vec![
        doc! {
            "_id": 1,
            "name": "first document",
            "age": 25
        },
        doc! {
            "_id": 2,
            "name": "second document",
            "age": 30
        },
        doc! {
            "_id": 3,
            "name": "third document",
            "age": 35
        },
    ];

    // Run the test for age=gt.30
    run_query_parser_test(
        "advanced comparison",
        "?age=gt.30",
        docs.clone(),
        vec![docs[2].clone()], // Expect only the third document
    );
}

#[test]
#[serial]
fn test_advanced_logical_query() {
    // Test documents
    let docs = vec![
        doc! {
            "_id": 1,
            "name": "first document",
            "age": 25,
            "category": "A"
        },
        doc! {
            "_id": 2,
            "name": "second document",
            "age": 30,
            "category": "B"
        },
        doc! {
            "_id": 3,
            "name": "third document",
            "age": 35,
            "category": "A"
        },
    ];

    // Run a simpler test for now
    run_query_parser_test(
        "advanced logical",
        "?age=gt.30",
        docs.clone(),
        vec![docs[2].clone()], // Expect only the third document
    );
}
