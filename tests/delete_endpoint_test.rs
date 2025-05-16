use mongodb::bson::{Document, doc};
use serde_json;
use serial_test::serial;

// Import test environment and utilities from utils module
mod utils;
use utils::test_environment::TestEnvironment;
use utils::utils::{make_delete_request, make_get_request};

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

// Helper function to set up a test collection and perform a DELETE operation
fn run_delete_test(
    env: &TestEnvironment,
    test_name: &str,
    initial_docs: Vec<Document>,
    query_params: &str,
    expected_deleted_count: u64,
) -> Vec<Document> {
    // Generate a unique collection name for this test
    let collection_name = unique_collection_name(test_name);

    // Insert initial test data
    env.insert_test_data(&collection_name, initial_docs);

    // Make a DELETE request to our endpoint
    let full_request_path = format!("/{}{}", collection_name, query_params);
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
    assert_eq!(
        delete_result["deletedCount"].as_u64().unwrap(),
        expected_deleted_count,
        "Expected {} documents to be deleted",
        expected_deleted_count
    );

    // Now make a GET request to verify the remaining documents
    let get_path = format!("/{}", collection_name);
    let (get_status_code, get_body) = make_get_request(&get_path);

    // Verify the GET response
    assert_eq!(
        get_status_code, 200,
        "Expected GET status code 200, got {}",
        get_status_code
    );

    // Parse the JSON response into BSON documents
    let documents: Vec<Document> =
        serde_json::from_str(&get_body).expect("Failed to parse JSON response");

    // Return the remaining documents for further verification
    documents
}

#[test]
#[serial]
fn test_delete_endpoint_all_cases() {
    // Create a single test environment for all test cases
    let env = TestEnvironment::new();

    // Test case 1: Delete a single document
    {
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

        // Run the delete test
        let remaining_docs = run_delete_test(
            &env,
            "delete_single_document",
            initial_docs,
            "?_id=1",
            1, // Expect 1 document to be deleted
        );

        // Verify only the second document remains
        assert_eq!(
            remaining_docs.len(),
            1,
            "Expected 1 document, got {}",
            remaining_docs.len()
        );
        assert_eq!(
            remaining_docs[0]["_id"].as_i32().unwrap(),
            2,
            "Expected document with _id=2"
        );
    }

    // Test case 2: Delete multiple documents
    {
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

        // Run the delete test
        let remaining_docs = run_delete_test(
            &env,
            "delete_multiple_documents",
            initial_docs,
            "?category=A",
            2, // Expect 2 documents to be deleted
        );

        // Verify only the third document remains
        assert_eq!(
            remaining_docs.len(),
            1,
            "Expected 1 document, got {}",
            remaining_docs.len()
        );
        assert_eq!(
            remaining_docs[0]["category"].as_str().unwrap(),
            "B",
            "Expected document with category=B"
        );
    }

    // Test case 3: Delete all documents
    {
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

        // Run the delete test
        let remaining_docs = run_delete_test(
            &env,
            "delete_all_documents",
            initial_docs,
            "", // Empty query string to delete all documents
            2,  // Expect 2 documents to be deleted
        );

        // Verify no documents remain
        assert_eq!(
            remaining_docs.len(),
            0,
            "Expected 0 documents, got {}",
            remaining_docs.len()
        );
    }
}
