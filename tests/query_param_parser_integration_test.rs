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

#[test]
#[serial]
fn test_query_parser_all_cases() {
    // Create a single test environment for all test cases
    let env = TestEnvironment::new();

    // Test case 1: Simple field value query
    {
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

        // Generate a unique collection name for this test
        let collection_name = unique_collection_name("simple_field_value");

        // Insert test data
        env.insert_test_data(&collection_name, docs.clone());

        // Make a request to our endpoint
        let full_request_path = format!("/{}?age=30", collection_name);
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

        // Verify the response matches expected documents
        assert_eq!(
            documents,
            vec![docs[1].clone()],
            "Documents don't match expected values"
        );
    }

    // Test case 2: Advanced comparison query
    {
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

        // Generate a unique collection name for this test
        let collection_name = unique_collection_name("advanced_comparison");

        // Insert test data
        env.insert_test_data(&collection_name, docs.clone());

        // Make a request to our endpoint
        let full_request_path = format!("/{}?age=gt.30", collection_name);
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

        // Verify the response matches expected documents
        assert_eq!(
            documents,
            vec![docs[2].clone()],
            "Documents don't match expected values"
        );
    }

    // Test case 3: Advanced logical query
    {
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

        // Generate a unique collection name for this test
        let collection_name = unique_collection_name("advanced_logical");

        // Insert test data
        env.insert_test_data(&collection_name, docs.clone());

        // Make a request to our endpoint
        let full_request_path = format!("/{}?age=gt.30", collection_name);
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

        // Verify the response matches expected documents
        assert_eq!(
            documents,
            vec![docs[2].clone()],
            "Documents don't match expected values"
        );
    }
}
