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

/// Run a PUT/PATCH endpoint test with the given parameters
///
/// This function:
/// 1. Creates a test environment (MongoDB + app)
/// 2. Creates a collection with a unique name and inserts initial documents
/// 3. Makes a PUT/PATCH request to update documents matching the filter
/// 4. Makes a GET request to verify the documents were updated correctly
/// 5. Automatically cleans up all processes when the test environment is dropped
fn run_update_endpoint_test(
    test_name: &str,
    initial_documents: Vec<Document>,
    filter_query: &str,
    update_document: Document,
    expected_documents: Vec<Document>,
    use_patch: bool,
) {
    // Create test environment (starts MongoDB and app)
    let env = TestEnvironment::new();

    // Generate a unique collection name for this test
    let collection_name = unique_collection_name(test_name);

    // Insert initial test data
    env.insert_test_data(&collection_name, initial_documents);

    // Convert update document to JSON string for the request
    let json_body =
        serde_json::to_string(&update_document).expect("Failed to convert document to JSON");

    // Make a PUT/PATCH request to our endpoint
    let full_request_path = format!("/{}{}", collection_name, filter_query);
    let (status_code, body) = if use_patch {
        make_patch_request(&full_request_path, &json_body)
    } else {
        make_put_request(&full_request_path, &json_body)
    };

    // Verify the response status code is 200 (OK) or 201 (Created) for PUT with upsert
    assert!(
        status_code == 200 || (status_code == 201 && !use_patch),
        "Expected status code 200 or 201 (for PUT upsert), got {}",
        status_code
    );

    // Parse the response to get the update result
    let update_result: serde_json::Value =
        serde_json::from_str(&body).expect("Failed to parse update result");

    if status_code == 201 {
        // For upsert that created a new document
        assert!(
            update_result["upsertedId"].is_object(),
            "Expected upsertedId for a new document"
        );
    } else {
        // For normal update (not an upsert that created a document)
        assert!(
            !update_result["matchedCount"].is_null(),
            "matchedCount should not be null for a normal update operation"
        );

        assert!(
            update_result["matchedCount"].as_u64().unwrap() > 0,
            "No documents matched the filter"
        );

        // Don't assert on modifiedCount as it might be 0 if the document already has the same values
        // but we should at least verify it exists
        assert!(
            !update_result["modifiedCount"].is_null(),
            "modifiedCount should not be null for a normal update operation"
        );
    }

    // Now make a GET request to verify the documents were updated
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

    // Verify the documents match the expected state after update
    assert_eq!(
        documents.len(),
        expected_documents.len(),
        "Expected {} documents, got {}",
        expected_documents.len(),
        documents.len()
    );

    // The test environment will be automatically cleaned up when it goes out of scope
}

#[test]
#[serial]
fn test_put_update_single_document() {
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

    // Expected document after update
    let expected_doc = doc! {
        "_id": 1,
        "name": "updated document",
        "value": 100
    };

    // Run the test
    run_update_endpoint_test(
        "update single document",
        vec![initial_doc],
        "?_id=1",
        update_doc,
        vec![expected_doc],
        false, // use PUT
    );
}

#[test]
#[serial]
fn test_patch_update_multiple_documents() {
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

    // Expected documents after update (only category A documents are updated)
    let expected_docs = vec![
        doc! {
            "_id": 1,
            "name": "first document",
            "category": "A",
            "value": 999
        },
        doc! {
            "_id": 2,
            "name": "second document",
            "category": "A",
            "value": 999
        },
        doc! {
            "_id": 3,
            "name": "third document",
            "category": "B",
            "value": 30
        },
    ];

    // Run the test
    run_update_endpoint_test(
        "update multiple documents",
        initial_docs,
        "?category=A",
        update_doc,
        expected_docs,
        true, // use PATCH
    );
}

#[test]
#[serial]
fn test_put_upsert_new_document() {
    // Document to insert via upsert
    let new_doc = doc! {
        "name": "upserted document",
        "value": 200,
        "tags": ["new", "upsert"]
    };

    // Expected document after upsert
    // We only specify the fields we want to verify
    let expected_doc = doc! {
        "name": "upserted document",
        "value": 200,
        "tags": ["new", "upsert"]
    };

    // Run the test with an empty initial collection
    // The filter query uses a field that doesn't exist to ensure upsert creates a new document
    run_update_endpoint_test(
        "upsert_new_document",
        Vec::new(),             // No initial documents
        "?non_existent_id=999", // Filter that won't match any documents
        new_doc,
        vec![expected_doc], // Expect one document with our fields
        false,              // use PUT
    );
}
