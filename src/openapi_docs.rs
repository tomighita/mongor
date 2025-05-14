#![allow(dead_code)]

use actix_web::{HttpResponse, Responder, web};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{
    OpenApi, ToSchema,
    openapi::path::{OperationBuilder, PathItemBuilder},
};

use crate::catalog::Catalog;
use crate::shared::AppState;

/// Structure to represent a MongoDB collection for OpenAPI docs
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CollectionInfo {
    name: String,
    options: HashMap<String, serde_json::Value>,
}

/// Generate a dynamically updated OpenAPI document based on the current database collections
#[derive(OpenApi)]
#[openapi(
    paths(
        get_collections,
        get_collection_data,
        ping
    ),
    components(
        schemas(CollectionInfo)
    ),
    tags(
        (name = "collections", description = "MongoDB Collections API"),
        (name = "system", description = "System endpoints")
    )
)]
pub struct ApiDoc;

/// Add custom paths for collections that are discovered at runtime
pub fn get_dynamic_openapi(catalog: &Catalog) -> utoipa::openapi::OpenApi {
    // Start with the base OpenAPI document
    let mut openapi = ApiDoc::openapi();

    // Add collection-specific paths
    for collection in &catalog.collection_specs {
        let collection_name = collection.name.clone();

        // 1. GET path for retrieving documents
        let get_path = format!("/api/collections/{}", collection_name);
        let get_path_item = PathItemBuilder::new()
            .operation(
                utoipa::openapi::HttpMethod::Get,
                OperationBuilder::new()
                    .description(Some("Test description"))
                    .response(
                        "200",
                        utoipa::openapi::ResponseBuilder::new()
                            .description("Documents retrieved successfully")
                            .content(
                                "application/json",
                                utoipa::openapi::ContentBuilder::new()
                                    // .schema(utoipa::openapi::Schema::from_json_schema(
                                    //     serde_json::json!({
                                    //         "type": "array",
                                    //         "items": {"type": "object"}
                                    //     }),
                                    // ))
                                    .example(Some(serde_json::json!([
                                        {"_id": "example_id_1", "field1": "value1"},
                                        {"_id": "example_id_2", "field2": 42}
                                    ])))
                                    .build(),
                            )
                            .build(),
                    )
                    .build(),
            )
            // .summary(format!(
            //     "Fetch documents from the {} collection",
            //     collection_name
            // ))
            // .description(format!(
            //     "Returns all documents from the {} collection, optionally filtered",
            //     collection_name
            // ))
            // .tag("collections")
            // // Add query parameters
            // .parameter(
            //     utoipa::openapi::ParameterBuilder::new()
            //         .name("limit")
            //         .schema(utoipa::openapi::Object::with_type(
            //             utoipa::openapi::SchemaType::Integer,
            //         ))
            //         .description("Maximum number of documents to return")
            //         .in_query()
            //         .build(),
            // )
            // .parameter(
            //     utoipa::openapi::ParameterBuilder::new()
            //         .name("skip")
            //         .schema(utoipa::openapi::Object::with_type(
            //             utoipa::openapi::SchemaType::Integer,
            //         ))
            //         .description("Number of documents to skip")
            //         .in_query()
            //         .build(),
            // )
            // // Add response with example
            // .response(
            //     "200",
            //     utoipa::openapi::ResponseBuilder::new()
            //         .description("Documents retrieved successfully")
            //         .content(
            //             "application/json",
            //             utoipa::openapi::ContentBuilder::new()
            //                 .schema(utoipa::openapi::Schema::from_json_schema(
            //                     serde_json::json!({
            //                         "type": "array",
            //                         "items": {"type": "object"}
            //                     }),
            //                 ))
            //                 .example(Some(serde_json::json!([
            //                     {"_id": "example_id_1", "field1": "value1"},
            //                     {"_id": "example_id_2", "field2": 42}
            //                 ])))
            //                 .build(),
            //         )
            //         .build(),
            // )
            // .response(
            //     "404",
            //     utoipa::openapi::ResponseBuilder::new().description("Collection not found"),
            // )
            .build();

        // Add this path to the OpenAPI document
        openapi.paths.paths.insert(get_path, get_path_item);
    }

    openapi
}

/// Endpoint to serve the dynamically generated OpenAPI document
#[actix_web::get("/openapi.json")]
pub async fn get_openapi_json(data: web::Data<AppState>) -> impl Responder {
    if let Some(catalog) = crate::catalog::get_cached_collections(&data) {
        let openapi = get_dynamic_openapi(&catalog);
        HttpResponse::Ok().json(openapi)
    } else {
        HttpResponse::InternalServerError().body("Could not access collections catalog")
    }
}

// These are skeleton documentation endpoints for the static parts of your API
// They match your existing API but will be augmented with dynamic routes

/// Get a list of all collections
#[utoipa::path(
    get,
    path = "/api/collections",
    tag = "collections",
    responses(
        (status = 200, description = "List of collections retrieved successfully", body = Vec<CollectionInfo>),
        (status = 500, description = "Failed to access collections")
    )
)]
async fn get_collections() {}

/// Get data from a specific collection
#[utoipa::path(
    get,
    path = "/api/collections/{collection_name}",
    tag = "collections",
    params(
        ("collection_name" = String, Path, description = "Name of the collection to retrieve")
    ),
    responses(
        (status = 200, description = "Collection data retrieved successfully"),
        (status = 404, description = "Collection not found"),
        (status = 500, description = "Failed to access collections")
    )
)]
async fn get_collection_data() {}

/// Ping the database
#[utoipa::path(
    get,
    path = "/api/ping",
    tag = "system",
    responses(
        (status = 200, description = "Database ping successful"),
        (status = 500, description = "Database ping failed")
    )
)]
async fn ping() {}
