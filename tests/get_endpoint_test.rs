use mongodb::bson::{Document, doc};
use serde_json;

// Import test environment from utils module
mod utils;
use utils::shared_test_environment::{SharedTestEnvironment, make_http_request};

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
/// 1. Uses the shared test environment (which ensures the database is set up properly)
/// 2. Creates a collection with a unique name and inserts test documents
/// 3. Makes a request to the endpoint
/// 4. Verifies the response matches the expected documents
fn run_get_endpoint_test(
    test_name: &str,
    request_path: &str,
    test_documents: Vec<Document>,
    expected_documents: Vec<Document>,
) {
    // Create test environment (setup is done automatically during initialization)
    let env = SharedTestEnvironment::new();

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

    // We don't need to run teardown after each test since we're using a shared MongoDB instance
    // The database will be cleaned up when all tests are done
}

#[test]
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
