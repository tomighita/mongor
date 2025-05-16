#![allow(dead_code)]

use actix_web::{HttpResponse, Responder, web};
use mongodb::bson::{Bson, Document};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::{
    OpenApi, ToSchema,
    openapi::{
        ContentBuilder, ObjectBuilder, RefOr, Required, Schema, SchemaFormat,
        path::{OperationBuilder, ParameterBuilder, ParameterIn},
        request_body::{RequestBody, RequestBodyBuilder},
    },
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
        let path = format!("/collections/{}", collection_name);
        let get_path_item = OperationBuilder::new()
            .summary(Some("Retrieve documents matching the query"))
            .description(Some("Test description"))
            .tag(format!(
                "MongoDB Collections {}",
                if collection.options.validator.is_some() {
                    "with validator"
                } else {
                    "without validator"
                }
            ))
            .parameter(
                ParameterBuilder::new()
                    .parameter_in(ParameterIn::Query)
                    .name("limit")
                    .description(Some("Maximum number of documents to return | default: 100"))
                    .schema(Some(
                        ObjectBuilder::new()
                            .schema_type(utoipa::openapi::Type::Integer)
                            .build(),
                    ))
                    .build(),
            )
            .parameter(
                ParameterBuilder::new()
                    .parameter_in(ParameterIn::Query)
                    .name("skip")
                    .description(Some("Number of documents to skip | default: 0"))
                    .schema(Some(
                        ObjectBuilder::new()
                            .schema_type(utoipa::openapi::Type::Integer)
                            .build(),
                    ))
                    .build(),
            )
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
            .build();

        let put_path_item = OperationBuilder::new()
            .summary(Some("Update documents matching the query"))
            .description(Some("Test description - Put"))
            .tag(format!(
                "MongoDB Collections {}",
                if collection.options.validator.is_some() {
                    "with validator"
                } else {
                    "without validator"
                }
            ))
            .request_body(
                collection
                    .options
                    .validator
                    .clone()
                    .and_then(|v| mongo_validator_to_openapi_request_body(&v).ok()),
            )
            .response(
                "200",
                utoipa::openapi::ResponseBuilder::new()
                    .description("Updated document successfully")
                    .content(
                        "application/json",
                        utoipa::openapi::ContentBuilder::new()
                            .example(Some(serde_json::json!({
                              "matchedCount": 1,
                              "modifiedCount": 1
                            })))
                            .build(),
                    )
                    .build(),
            )
            .build();

        let post_path_item = OperationBuilder::new()
            .summary(Some("Create document"))
            .description(Some("Test description - Post"))
            .tag(format!(
                "MongoDB Collections {}",
                if collection.options.validator.is_some() {
                    "with validator"
                } else {
                    "without validator"
                }
            ))
            .request_body(
                collection
                    .options
                    .validator
                    .clone()
                    .and_then(|v| mongo_validator_to_openapi_request_body(&v).ok()),
            )
            .response(
                "200",
                utoipa::openapi::ResponseBuilder::new()
                    .description("Created document successfully")
                    .content(
                        "application/json",
                        utoipa::openapi::ContentBuilder::new()
                            .example(Some(serde_json::json!({
                              "$oid": "682736d3fb21114a6a908f17"
                            })))
                            .build(),
                    )
                    .build(),
            )
            .build();

        let delete_path_item = OperationBuilder::new()
            .summary(Some("Delete documents matching the query"))
            .description(Some("Test description - Delete"))
            .tag(format!(
                "MongoDB Collections {}",
                if collection.options.validator.is_some() {
                    "with validator"
                } else {
                    "without validator"
                }
            ))
            .parameter(
                ParameterBuilder::new()
                    .parameter_in(ParameterIn::Query)
                    .name("limit")
                    .description(Some("Maximum number of documents to return | default: 100"))
                    .schema(Some(
                        ObjectBuilder::new()
                            .schema_type(utoipa::openapi::Type::Integer)
                            .build(),
                    ))
                    .build(),
            )
            .parameter(
                ParameterBuilder::new()
                    .parameter_in(ParameterIn::Query)
                    .name("skip")
                    .description(Some("Number of documents to skip | default: 0"))
                    .schema(Some(
                        ObjectBuilder::new()
                            .schema_type(utoipa::openapi::Type::Integer)
                            .build(),
                    ))
                    .build(),
            )
            .response(
                "200",
                utoipa::openapi::ResponseBuilder::new()
                    .description("Documents deleted successfully")
                    .content(
                        "application/json",
                        utoipa::openapi::ContentBuilder::new()
                            .example(Some(serde_json::json!({
                              "deletedCount": 2
                            })))
                            .build(),
                    )
                    .build(),
            )
            .build();

        openapi.paths.add_path_operation(
            path.clone(),
            vec![utoipa::openapi::HttpMethod::Get],
            get_path_item,
        );
        openapi.paths.add_path_operation(
            path.clone(),
            vec![utoipa::openapi::HttpMethod::Post],
            post_path_item,
        );
        openapi.paths.add_path_operation(
            path.clone(),
            vec![utoipa::openapi::HttpMethod::Put],
            put_path_item,
        );
        openapi.paths.add_path_operation(
            path.clone(),
            vec![utoipa::openapi::HttpMethod::Delete],
            delete_path_item,
        );
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

// Helper function to convert a BSON schema for a single property
// into an OpenAPI Schema object.
// Helper function to convert a BSON schema for a single property or an entire document
// into an OpenAPI Schema object. This function is largely the same as in the previous context.
fn bson_schema_to_openapi_schema(
    schema_name_or_property: &str, // For context in error messages or logging
    bson_schema_doc: &Document,
) -> Result<RefOr<Schema>, String> {
    // Get the BSON type (e.g., "string", "int", "object")
    let bson_type_str = bson_schema_doc.get_str("bsonType").map_err(|e| {
        format!(
            "Schema/Property '{}': Missing or invalid 'bsonType': {}",
            schema_name_or_property, e
        )
    })?;

    // Use ObjectBuilder from utoipa to construct the OpenAPI schema
    let mut schema_builder = ObjectBuilder::new();

    // Map BSON types and constraints to OpenAPI schema properties
    match bson_type_str {
        "string" => {
            let mut is_purely_string_compatible_enum = true;
            let mut has_enum_key = false;

            if let Ok(enum_values_bson) = bson_schema_doc.get_array("enum") {
                has_enum_key = true;
                for b_val in enum_values_bson {
                    if !matches!(b_val, Bson::String(_) | Bson::Null) {
                        is_purely_string_compatible_enum = false;
                        break;
                    }
                }
            }

            if has_enum_key && !is_purely_string_compatible_enum {
                schema_builder = schema_builder.schema_type(utoipa::openapi::Type::String); // Use Value for mixed-type enums
                let original_description = bson_schema_doc.get_str("description").ok();
                let mixed_enum_note = " (Note: enum may contain non-string values)";
                let combined_description = match original_description {
                    Some(desc) => format!("{}{}", desc, mixed_enum_note),
                    None => mixed_enum_note.to_string(),
                };
                schema_builder = schema_builder.description(Some(combined_description));
            } else {
                schema_builder = schema_builder.schema_type(utoipa::openapi::Type::String);
            }

            if let Ok(min_length) = bson_schema_doc.get_i64("minLength") {
                schema_builder = schema_builder.min_length(Some(min_length as usize));
            }
            if let Ok(max_length) = bson_schema_doc.get_i64("maxLength") {
                schema_builder = schema_builder.max_length(Some(max_length as usize));
            }
            if let Ok(pattern) = bson_schema_doc.get_str("pattern") {
                schema_builder = schema_builder.pattern(Some(pattern.to_string()));
            }

            if let Ok(enum_values_bson) = bson_schema_doc.get_array("enum") {
                let enums: Vec<serde_json::Value> = enum_values_bson
                    .iter()
                    .filter_map(|b_val| match b_val {
                        Bson::String(s) => Some(serde_json::Value::String(s.clone())),
                        Bson::Int32(i) => Some(serde_json::Value::Number((*i).into())),
                        Bson::Int64(l) => Some(serde_json::Value::Number((*l).into())),
                        Bson::Double(d) => {
                            serde_json::Number::from_f64(*d).map(serde_json::Value::Number)
                        }
                        Bson::Boolean(b) => Some(serde_json::Value::Bool(*b)),
                        Bson::Null => Some(serde_json::Value::Null),
                        _ => None,
                    })
                    .collect();
                if !enums.is_empty() {
                    schema_builder = schema_builder.enum_values(Some(enums));
                }
            }

            if let Ok("date") = bson_schema_doc.get_str("format") {
                schema_builder = schema_builder.format(Some(SchemaFormat::KnownFormat(
                    utoipa::openapi::KnownFormat::Date,
                )));
            } else if let Ok("date-time") = bson_schema_doc.get_str("format") {
                schema_builder = schema_builder.format(Some(SchemaFormat::KnownFormat(
                    utoipa::openapi::KnownFormat::DateTime,
                )));
            } else if let Ok("byte") = bson_schema_doc.get_str("format") {
                schema_builder = schema_builder.format(Some(SchemaFormat::KnownFormat(
                    utoipa::openapi::KnownFormat::Byte,
                )));
            }
        }
        "int" | "long" => {
            schema_builder = schema_builder.schema_type(utoipa::openapi::Type::Integer);
            if bson_type_str == "long" {
                schema_builder = schema_builder.format(Some(SchemaFormat::KnownFormat(
                    utoipa::openapi::KnownFormat::Int64,
                )));
            } else {
                schema_builder = schema_builder.format(Some(SchemaFormat::KnownFormat(
                    utoipa::openapi::KnownFormat::Int32,
                )));
            }
            if let Ok(min) = bson_schema_doc.get_i32("minimum") {
                // BSON int can be i32
                schema_builder = schema_builder.minimum(Some(min as f64));
            } else if let Ok(min) = bson_schema_doc.get_i64("minimum") {
                // or i64
                schema_builder = schema_builder.minimum(Some(min as f64));
            }
            if let Ok(max) = bson_schema_doc.get_i32("maximum") {
                schema_builder = schema_builder.maximum(Some(max as f64));
            } else if let Ok(max) = bson_schema_doc.get_i64("maximum") {
                schema_builder = schema_builder.maximum(Some(max as f64));
            }
        }
        "double" | "decimal" => {
            schema_builder = schema_builder.schema_type(utoipa::openapi::Type::Number);
            if bson_type_str == "double" {
                schema_builder = schema_builder.format(Some(SchemaFormat::KnownFormat(
                    utoipa::openapi::KnownFormat::Double,
                )));
            }
            // For "decimal", it's often represented as a string in OpenAPI to preserve precision,
            // or as a number if some precision loss is acceptable.
            // This example treats it as a generic number.
            if let Ok(min) = bson_schema_doc.get_f64("minimum") {
                schema_builder = schema_builder.minimum(Some(min));
            } else if let Ok(min_dec_str) = bson_schema_doc.get_str("minimum") {
                // For Decimal128 as string
                if let Ok(min_val) = min_dec_str.parse::<f64>() {
                    schema_builder = schema_builder.minimum(Some(min_val));
                }
            }
            if let Ok(max) = bson_schema_doc.get_f64("maximum") {
                schema_builder = schema_builder.maximum(Some(max));
            } else if let Ok(max_dec_str) = bson_schema_doc.get_str("maximum") {
                if let Ok(max_val) = max_dec_str.parse::<f64>() {
                    schema_builder = schema_builder.maximum(Some(max_val));
                }
            }
        }
        "bool" => {
            schema_builder = schema_builder.schema_type(utoipa::openapi::Type::Boolean);
        }
        "date" => {
            // BSON date type
            schema_builder = schema_builder
                .schema_type(utoipa::openapi::Type::String)
                .format(Some(SchemaFormat::KnownFormat(
                    utoipa::openapi::KnownFormat::DateTime,
                )));
        }
        "objectId" => {
            // MongoDB ObjectId
            schema_builder = schema_builder
                .schema_type(utoipa::openapi::Type::String)
                .pattern(Some("^[a-f\\d]{24}$".to_string())) // Typical ObjectId hex pattern
                .description(Some(
                    "MongoDB ObjectId (24-character hex string)".to_string(),
                ));
        }
        "array" => {
            // schema_builder = schema_builder.schema_type(utoipa::openapi::Type::Array);
            // if let Ok(items_schema_doc) = bson_schema_doc.get_document("items") {
            // Recursively convert the schema for array items
            // let items_openapi_schema = bson_schema_to_openapi_schema(
            //     &format!("{}[items]", schema_name_or_property),
            //     items_schema_doc,
            // )?;
            // TODO: Handle array items schema
            // schema_builder = schema_builder.items(Some(items_openapi_schema));
            // }
        }
        "object" => {
            schema_builder = schema_builder.schema_type(utoipa::openapi::Type::Object);
            // Process nested properties for the object
            if let Ok(object_properties) = bson_schema_doc.get_document("properties") {
                for (key, value_doc) in object_properties.iter() {
                    if let Bson::Document(prop_doc) = value_doc {
                        // Recursively convert schema for each property
                        match bson_schema_to_openapi_schema(
                            &format!("{}.{}", schema_name_or_property, key),
                            prop_doc,
                        ) {
                            Ok(prop_schema) => {
                                schema_builder = schema_builder.property(key, prop_schema);
                            }
                            Err(e) => {
                                // Log error for sub-property conversion
                                eprintln!(
                                    "Warning: Skipping sub-property {} for object {}: {}",
                                    key, schema_name_or_property, e
                                );
                            }
                        }
                    }
                }
            }
        }
        "null" => {
            schema_builder = schema_builder.schema_type(utoipa::openapi::Type::Null);
        }
        unsupported => {
            return Err(format!(
                "Schema/Property '{}': Unsupported bsonType '{}' for OpenAPI schema conversion.",
                schema_name_or_property, unsupported
            ));
        }
    }

    // Set title for the schema
    if let Ok(title) = bson_schema_doc.get_str("title") {
        schema_builder = schema_builder.title(Some(title.to_string()));
    }

    Ok(RefOr::T(Schema::Object(schema_builder.build())))
}

/// Transforms a MongoDB validator schema (bson::Document) into an utoipa OpenAPI RequestBody.
///
/// # Arguments
///
/// * `validator_doc` - A reference to a `mongodb::bson::Document` representing the MongoDB $jsonSchema validator.
///   This document itself is expected to be a valid JSON Schema object.
/// * `media_type_str` - The media type for the request body content, e.g., "application/json".
/// * `is_required` - A boolean indicating if the request body is required for the operation.
///
/// # Returns
///
/// * `Result<utoipa::openapi::request_body::RequestBody, String>` - An `RequestBody` object on success,
///   or an error string on failure.
pub fn mongo_validator_to_openapi_request_body(
    validator_doc: &Document,
) -> Result<RequestBody, String> {
    // Convert the entire MongoDB validator document into an OpenAPI Schema.
    // The validator_doc is treated as the root schema for the request body.
    let openapi_schema = bson_schema_to_openapi_schema("RequestBodyRootSchema", validator_doc).ok();

    let content = ContentBuilder::new().schema(openapi_schema).build(); // Wrap the map in the Content struct.

    // Build the RequestBody object.
    let mut request_body_builder = RequestBodyBuilder::new().content("application/json", content);

    // Set the description for the RequestBody from the validator's title or description.
    if let Ok(title) = validator_doc.get_str("title") {
        request_body_builder = request_body_builder.description(Some(title.to_string()));
    } else if let Ok(description) = validator_doc.get_str("description") {
        request_body_builder = request_body_builder.description(Some(description.to_string()));
    }

    // Set if the request body is required for the operation.
    request_body_builder = request_body_builder.required(Some(Required::True));

    Ok(request_body_builder.build())
}
