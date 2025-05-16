use mongodb::bson::{Document, doc};
use serde_json;
use serial_test::serial;

// Import test environment and utilities from utils module
mod utils;
use utils::test_environment::TestEnvironment;
use utils::utils::{make_http_request, make_patch_request, make_put_request};

// Base name for test collections
static TEST_COLLECTION_BASE_NAME: &str = "mongor_put_endpoint_test";

// Generate a unique collection name for each test
fn unique_collection_name(test_name: &str) -> String {
    format!(
        "{}_{}",
        TEST_COLLECTION_BASE_NAME,
        test_name.replace(" ", "_")
    )
}

// Common helper function for update operations (PUT, PATCH)
fn run_update_test(
    env: &TestEnvironment,
    test_name: &str,
    initial_docs: Vec<Document>,
    query_params: &str,
    update_doc: Document,
    http_method: &str, // "PUT" or "PATCH"
    expected_status: u16,
) -> (serde_json::Value, Vec<Document>) {
    // Generate a unique collection name for this test
    let collection_name = unique_collection_name(test_name);

    // Insert initial test data
    env.insert_test_data(&collection_name, initial_docs);

    // Convert update document to JSON string for the request
    let json_body = serde_json::to_string(&update_doc).expect("Failed to convert document to JSON");

    // Make the request to our endpoint
    let full_request_path = format!("/{}{}", collection_name, query_params);
    let (status_code, body) = match http_method {
        "PUT" => make_put_request(&full_request_path, &json_body),
        "PATCH" => make_patch_request(&full_request_path, &json_body),
        _ => panic!("Unsupported HTTP method: {}", http_method),
    };

    // Verify the response status code
    assert_eq!(
        status_code, expected_status,
        "Expected status code {}, got {}",
        expected_status, status_code
    );

    // Parse the response to get the update result
    let update_result: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse update result");

    // Now make a GET request to verify the documents
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

    // Return both the update result and the documents for further verification
    (update_result, documents)
}

// Helper function to set up a test collection and perform a PUT operation
fn run_put_test(
    env: &TestEnvironment,
    test_name: &str,
    initial_docs: Vec<Document>,
    query_params: &str,
    update_doc: Document,
    expected_status: u16,
    expected_matched_count: u64,
) -> Vec<Document> {
    // Run the common update test
    let (update_result, documents) = run_update_test(
        env,
        test_name,
        initial_docs,
        query_params,
        update_doc,
        "PUT",
        expected_status,
    );

    // Verify that the update was successful
    if expected_status == 200 {
        // For updates
        assert_eq!(
            update_result["matchedCount"].as_u64().unwrap(),
            expected_matched_count,
            "Expected {} documents to match",
            expected_matched_count
        );
    } else if expected_status == 201 {
        // For upserts
        assert!(
            update_result["upsertedId"].is_object(),
            "Expected upsertedId for a new document"
        );
    }

    // Return the documents for further verification
    documents
}

// Helper function to set up a test collection and perform a PATCH operation
fn run_patch_test(
    env: &TestEnvironment,
    test_name: &str,
    initial_docs: Vec<Document>,
    query_params: &str,
    update_doc: Document,
    expected_matched_count: u64,
    expected_modified_count: u64,
) -> Vec<Document> {
    // Run the common update test
    let (update_result, documents) = run_update_test(
        env,
        test_name,
        initial_docs,
        query_params,
        update_doc,
        "PATCH",
        200, // PATCH always expects 200 OK
    );

    // Verify that the update was successful
    assert_eq!(
        update_result["matchedCount"].as_u64().unwrap(),
        expected_matched_count,
        "Expected {} documents to match",
        expected_matched_count
    );
    assert_eq!(
        update_result["modifiedCount"].as_u64().unwrap(),
        expected_modified_count,
        "Expected {} documents to be modified",
        expected_modified_count
    );

    // Return the documents for further verification
    documents
}

#[test]
#[serial]
fn test_put_endpoint_all_cases() {
    // Create a single test environment for all test cases
    let env = TestEnvironment::new();

    // Test case 1: PUT update single document
    {
        // Initial document
        let initial_doc = doc! {
            "_id": 1,
            "name": "original document",
            "value": 42
        };

        // Update document
        let update_doc = doc! {
            "name": "updated document",
            "value": 100
        };

        // Run the PUT test
        let documents = run_put_test(
            &env,
            "update_single_document",
            vec![initial_doc],
            "?_id=1",
            update_doc,
            200, // Expected status code
            1,   // Expected matched count
        );

        // Verify the documents match the expected state after update
        assert_eq!(
            documents.len(),
            1,
            "Expected 1 document, got {}",
            documents.len()
        );
        assert_eq!(
            documents[0]["name"].as_str().unwrap(),
            "updated document",
            "Document name not updated"
        );
        assert_eq!(
            documents[0]["value"].as_i32().unwrap(),
            100,
            "Document value not updated"
        );
    }

    // Test case 2: PATCH update multiple documents
    {
        // Initial documents
        let initial_docs = vec![
            doc! {
                "_id": 1,
                "name": "first document",
                "category": "A",
                "value": 10
            },
            doc! {
                "_id": 2,
                "name": "second document",
                "category": "A",
                "value": 20
            },
            doc! {
                "_id": 3,
                "name": "third document",
                "category": "B",
                "value": 30
            },
        ];

        // Update document
        let update_doc = doc! {
            "value": 999
        };

        // Run the PATCH test
        let documents = run_patch_test(
            &env,
            "update_multiple_documents",
            initial_docs,
            "?category=A",
            update_doc,
            2, // Expected matched count
            2, // Expected modified count
        );

        // Verify the documents match the expected state after update
        assert_eq!(
            documents.len(),
            3,
            "Expected 3 documents, got {}",
            documents.len()
        );

        // Check that category A documents have been updated
        for doc in &documents {
            if doc["category"].as_str().unwrap() == "A" {
                assert_eq!(
                    doc["value"].as_i32().unwrap(),
                    999,
                    "Category A document not updated"
                );
            } else {
                assert_eq!(
                    doc["value"].as_i32().unwrap(),
                    30,
                    "Category B document should not be updated"
                );
            }
        }
    }

    // Test case 3: PUT upsert new document
    {
        // Document to insert via upsert
        let new_doc = doc! {
            "name": "upserted document",
            "value": 200,
            "tags": ["new", "upsert"]
        };

        // Run the PUT test for upsert
        let documents = run_put_test(
            &env,
            "upsert_new_document",
            Vec::new(), // Empty initial collection
            "?non_existent_id=999",
            new_doc.clone(),
            201, // Expected status code for upsert
            0,   // Expected matched count (0 for upsert)
        );

        // Verify the document was inserted
        assert_eq!(
            documents.len(),
            1,
            "Expected 1 document, got {}",
            documents.len()
        );
        assert_eq!(
            documents[0]["name"].as_str().unwrap(),
            "upserted document",
            "Document name not correct"
        );
        assert_eq!(
            documents[0]["value"].as_i32().unwrap(),
            200,
            "Document value not correct"
        );
    }
}
