use mongodb::bson::{Document, doc};
use serde_json;
use serial_test::serial;

// Import test environment and utilities from utils module
mod utils;
use utils::test_environment::TestEnvironment;
use utils::utils::make_get_request;

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

// Helper function to set up a test collection and perform a GET operation
fn run_get_test(
    env: &TestEnvironment,
    test_name: &str,
    test_docs: Vec<Document>,
    query_params: &str,
    expected_docs: Vec<Document>,
) {
    // Generate a unique collection name for this test
    let collection_name = unique_collection_name(test_name);

    // Insert test data
    env.insert_test_data(&collection_name, test_docs);

    // Make a request to our endpoint
    let full_request_path = format!("/{}{}", collection_name, query_params);
    let (status_code, body) = make_get_request(&full_request_path);

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
        documents, expected_docs,
        "Documents don't match expected values"
    );
}

#[test]
#[serial]
fn test_get_endpoint_all_cases() {
    // Create a single test environment for all test cases
    let env = TestEnvironment::new();

    // Test case 1: Get with matching document
    {
        // Test document
        let test_doc = doc! {
            "_id": 1,
            "name": "test document"
        };

        // Run the get test
        run_get_test(
            &env,
            "matching_document",
            vec![test_doc.clone()],
            "?_id=1",
            vec![test_doc],
        );
    }

    // Test case 2: Get with non-existent document
    {
        // Test document (to ensure collection exists)
        let test_doc = doc! {
            "_id": 1,
            "name": "test document"
        };

        // Run the get test
        run_get_test(
            &env,
            "non_existent_document",
            vec![test_doc],
            "?_id=999",
            Vec::new(), // Expect empty array
        );
    }

    // Test case 3: Get with multiple documents
    {
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

        // Run the get test
        run_get_test(
            &env,
            "multiple_documents",
            docs.clone(),
            "?category=A",
            vec![docs[0].clone(), docs[1].clone()],
        );
    }
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

        // Run the get test
        run_get_test(
            &env,
            "simple_field_value",
            docs.clone(),
            "?age=30",
            vec![docs[1].clone()],
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

        // Run the get test
        run_get_test(
            &env,
            "advanced_comparison",
            docs.clone(),
            "?age=gt.30",
            vec![docs[2].clone()],
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

        // Run the get test
        run_get_test(
            &env,
            "advanced_logical",
            docs.clone(),
            "?age=gt.30",
            vec![docs[2].clone()],
        );
    }

    // Test case 4: Complex nested query with and/or predicates
    {
        // Test documents with more fields for complex filtering
        let docs = vec![
            doc! {
                "_id": 1,
                "name": "first document",
                "age": 25,
                "category": "A",
                "status": "active",
                "score": 85
            },
            doc! {
                "_id": 2,
                "name": "second document",
                "age": 30,
                "category": "B",
                "status": "inactive",
                "score": 92
            },
            doc! {
                "_id": 3,
                "name": "third document",
                "age": 35,
                "category": "A",
                "status": "active",
                "score": 78
            },
            doc! {
                "_id": 4,
                "name": "fourth document",
                "age": 40,
                "category": "C",
                "status": "active",
                "score": 95
            },
            doc! {
                "_id": 5,
                "name": "fifth document",
                "age": 28,
                "category": "B",
                "status": "active",
                "score": 88
            },
        ];

        // Complex query: (category=A AND score<80) OR (age>35 AND status=active)
        // This should match document 3 (category A, score 78) and document 4 (age 40, status active)
        run_get_test(
            &env,
            "complex_nested_query",
            docs.clone(),
            "?or=(and=(category.A,score.lt.80),and=(age.gt.35,status.active))",
            vec![docs[2].clone(), docs[3].clone()],
        );
    }
}
