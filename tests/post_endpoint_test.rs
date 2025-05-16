use mongodb::bson::{Document, doc};
use serde_json;
use serial_test::serial;

// Import test environment and utilities from utils module
mod utils;
use utils::test_environment::TestEnvironment;
use utils::utils::{make_get_request, make_post_request};

// Base name for test collections
static TEST_COLLECTION_BASE_NAME: &str = "mongor_post_endpoint_test";

// Generate a unique collection name for each test
fn unique_collection_name(test_name: &str) -> String {
    format!(
        "{}_{}",
        TEST_COLLECTION_BASE_NAME,
        test_name.replace(" ", "_")
    )
}

// Helper function to set up a test collection and perform a POST operation
fn run_post_test(env: &TestEnvironment, test_name: &str, document: Document) {
    // Generate a unique collection name for this test
    let collection_name = unique_collection_name(test_name);

    // Ensure the collection exists but is empty
    env.insert_test_data(&collection_name, Vec::new());

    // Convert document to JSON string for the request
    let json_body = serde_json::to_string(&document).expect("Failed to convert document to JSON");

    // Make a POST request to our endpoint
    let full_request_path = format!("/{}", collection_name);
    let (status_code, _body) = make_post_request(&full_request_path, &json_body);

    // Verify the response status code is 201 (Created)
    assert_eq!(
        status_code, 201,
        "Expected status code 201, got {}",
        status_code
    );

    // Now make a GET request to verify the document was inserted
    let (get_status_code, get_body) = make_get_request(&full_request_path);

    // Verify the GET response
    assert_eq!(
        get_status_code, 200,
        "Expected GET status code 200, got {}",
        get_status_code
    );

    // Parse the JSON response into BSON documents
    let documents: Vec<Document> =
        serde_json::from_str(&get_body).expect("Failed to parse JSON response");

    // Verify the document was inserted
    assert_eq!(
        documents.len(),
        1,
        "Expected 1 document, got {}",
        documents.len()
    );
}

#[test]
#[serial]
fn test_post_endpoint_all_cases() {
    // Create a single test environment for all test cases
    let env = TestEnvironment::new();

    // Test case 1: Post a simple document
    {
        // Test document to insert
        let test_doc = doc! {
            "name": "test document",
            "value": 42
        };

        // Run the post test
        run_post_test(&env, "simple_document", test_doc);
    }

    // Test case 2: Post a complex document
    {
        // Test document with nested fields
        let test_doc = doc! {
            "name": "complex document",
            "nested": {
                "field1": "value1",
                "field2": 123
            },
            "array": ["item1", "item2", "item3"]
        };

        // Run the post test
        run_post_test(&env, "complex_document", test_doc);
    }
}
