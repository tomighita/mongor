use mongodb::bson::{Document, doc};
use serde_json;
use serial_test::serial;

// Import test environment and utilities from utils module
mod utils;
use utils::test_environment::TestEnvironment;
use utils::utils::{make_http_request, make_delete_request};

// Base name for test collections
static TEST_COLLECTION_BASE_NAME: &str = "mongor_delete_endpoint_test";

// Generate a unique collection name for each test
fn unique_collection_name(test_name: &str) -> String {
    format!(
        "{}_{}",
        TEST_COLLECTION_BASE_NAME,
        test_name.replace(" ", "_")
    )
}

/// Run a DELETE endpoint test with the given parameters
///
/// This function:
/// 1. Creates a test environment (MongoDB + app)
/// 2. Creates a collection with a unique name and inserts initial documents
/// 3. Makes a DELETE request to delete documents matching the filter
/// 4. Makes a GET request to verify the documents were deleted correctly
/// 5. Automatically cleans up all processes when the test environment is dropped
fn run_delete_endpoint_test(
    test_name: &str,
    initial_documents: Vec<Document>,
    filter_query: &str,
    expected_remaining_documents: Vec<Document>,
) {
    // Create test environment (starts MongoDB and app)
    let env = TestEnvironment::new();

    // Generate a unique collection name for this test
    let collection_name = unique_collection_name(test_name);

    // Insert initial test data
    env.insert_test_data(&collection_name, initial_documents);

    // Make a DELETE request to our endpoint
    let full_request_path = format!("/{}{}", collection_name, filter_query);
    let (status_code, body) = make_delete_request(&full_request_path);

    // Verify the response status code is 200 (OK)
    assert_eq!(
        status_code, 200,
        "Expected status code 200, got {}",
        status_code
    );

    // Parse the response to get the delete result
    let delete_result: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse delete result");
    
    // Verify that the delete was successful
    assert!(delete_result["deletedCount"].as_u64().unwrap() > 0, 
        "No documents were deleted");

    // Now make a GET request to verify the remaining documents
    let get_path = format!("/{}", collection_name);
    let (get_status_code, get_body) = make_http_request(&get_path);

    // Verify the GET response
    assert_eq!(
        get_status_code, 200,
        "Expected GET status code 200, got {}",
        get_status_code
    );

    // Parse the JSON response into BSON documents
    let documents: Vec<Document> =
        serde_json::from_str(&get_body).expect("Failed to parse JSON response");

    // Verify the remaining documents match the expected state after deletion
    assert_eq!(
        documents.len(), expected_remaining_documents.len(),
        "Expected {} documents, got {}",
        expected_remaining_documents.len(), documents.len()
    );

    // The test environment will be automatically cleaned up when it goes out of scope
}

#[test]
#[serial]
fn test_delete_single_document() {
    // Initial documents
    let initial_docs = vec![
        doc! {
            "_id": 1,
            "name": "first document"
        },
        doc! {
            "_id": 2,
            "name": "second document"
        },
    ];

    // Run the test - delete document with _id=1
    run_delete_endpoint_test(
        "delete single document",
        initial_docs.clone(),
        "?_id=1",
        vec![initial_docs[1].clone()], // Only the second document should remain
    );
}

#[test]
#[serial]
fn test_delete_multiple_documents() {
    // Initial documents
    let initial_docs = vec![
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

    // Run the test - delete all documents with category=A
    run_delete_endpoint_test(
        "delete multiple documents",
        initial_docs.clone(),
        "?category=A",
        vec![initial_docs[2].clone()], // Only the third document should remain
    );
}

#[test]
#[serial]
fn test_delete_all_documents() {
    // Initial documents
    let initial_docs = vec![
        doc! {
            "_id": 1,
            "name": "first document"
        },
        doc! {
            "_id": 2,
            "name": "second document"
        },
    ];

    // Run the test - delete all documents (empty filter)
    run_delete_endpoint_test(
        "delete all documents",
        initial_docs,
        "",
        Vec::new(), // No documents should remain
    );
}
