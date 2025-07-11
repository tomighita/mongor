use actix_web::{HttpResponse, Responder, delete, get, patch, post, put, web};
use futures_util::TryStreamExt;
use mongodb::bson::doc;
use serde_json::Value;

use crate::{query_param_parser::parse_find_options, shared::AppState};
use mongor::parse_match_query_params;

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[get("/ping")]
async fn ping(data: web::Data<AppState>) -> impl Responder {
    // Ping database and match on ping response
    match data
        .db_client
        .database(&data.config.database_name)
        .run_command(doc! {"ping": 1})
        .await
    {
        Ok(doc) => HttpResponse::Ok().body(format!("Pong! {}", doc)),
        Err(e) => {
            println!("Error pinging database: {:?}", e);
            HttpResponse::InternalServerError().body("Error pinging database")
        }
    }
}

#[get("/{coll_name}")]
async fn query_collection(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    data: web::Data<AppState>,
) -> impl Responder {
    let coll_name = path.into_inner();

    if let Some(e) = get_exception_if_collection_absent(coll_name.as_str(), &data).await {
        return e;
    }

    // Parse query parameters
    let filter = match parse_match_query_params(&query) {
        Ok(filter) => filter,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Invalid query parameter: {}", e));
        }
    };

    // Execute the query
    match data
        .db_client
        .database(&data.config.database_name)
        .collection::<mongodb::bson::Document>(&coll_name)
        .find(filter)
        .with_options(parse_find_options(&query))
        .await
    {
        Ok(cursor) => {
            // Convert cursor to vector of documents
            match cursor.try_collect::<Vec<mongodb::bson::Document>>().await {
                Ok(docs) => HttpResponse::Ok().json(docs),
                Err(e) => {
                    println!("Error collecting documents: {:?}", e);
                    HttpResponse::InternalServerError()
                        .body(format!("Error collecting documents: {:?}", e))
                }
            }
        }
        Err(e) => {
            println!("Error executing query: {:?}", e);
            HttpResponse::InternalServerError().body(format!("Error executing query: {:?}", e))
        }
    }
}

#[post("/{coll_name}")]
async fn insert_document(
    path: web::Path<String>,
    payload: web::Json<Value>,
    data: web::Data<AppState>,
) -> impl Responder {
    let coll_name = path.into_inner();

    if let Some(e) = get_exception_if_collection_absent(coll_name.as_str(), &data).await {
        return e;
    }

    // Convert the JSON payload to a MongoDB document
    let document = match mongodb::bson::to_document(&payload) {
        Ok(doc) => doc,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Invalid document format: {}", e));
        }
    };

    // Insert the document
    match data
        .db_client
        .database(&data.config.database_name)
        .collection::<mongodb::bson::Document>(&coll_name)
        .insert_one(document)
        .await
    {
        Ok(result) => HttpResponse::Created().json(result.inserted_id),
        Err(e) => {
            println!("Error inserting document: {:?}", e);
            HttpResponse::InternalServerError().body(format!("Error inserting document: {:?}", e))
        }
    }
}

#[put("/{coll_name}")]
async fn update_document(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    payload: web::Json<Value>,
    data: web::Data<AppState>,
) -> impl Responder {
    let coll_name = path.into_inner();

    if let Some(e) = get_exception_if_collection_absent(coll_name.as_str(), &data).await {
        return e;
    }

    // Parse query parameters for filter
    let filter = match parse_match_query_params(&query) {
        Ok(filter) => filter,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Invalid query parameter: {}", e));
        }
    };

    // Convert the JSON payload to a MongoDB document
    let update_doc = match mongodb::bson::to_document(&payload) {
        Ok(doc) => doc,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Invalid document format: {}", e));
        }
    };

    // Create the update document with $set operator
    let update = doc! { "$set": update_doc };

    // Update a single document with upsert
    match data
        .db_client
        .database(&data.config.database_name)
        .collection::<mongodb::bson::Document>(&coll_name)
        .update_one(filter, update)
        .upsert(true)
        .await
    {
        Ok(result) => {
            // Return 201 Created if a new document was inserted, otherwise 200 OK
            if result.upserted_id.is_some() {
                HttpResponse::Created().json(result)
            } else {
                HttpResponse::Ok().json(result)
            }
        }
        Err(e) => {
            println!("Error updating document: {:?}", e);
            HttpResponse::InternalServerError().body(format!("Error updating document: {:?}", e))
        }
    }
}

#[patch("/{coll_name}")]
async fn patch_document(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    payload: web::Json<Value>,
    data: web::Data<AppState>,
) -> impl Responder {
    // PATCH is the same as PUT in this implementation
    // We need to reimplement the logic here since we can't call the handler directly
    let coll_name = path.into_inner();

    if let Some(e) = get_exception_if_collection_absent(coll_name.as_str(), &data).await {
        return e;
    }

    // Parse query parameters for filter
    let filter = match parse_match_query_params(&query) {
        Ok(filter) => filter,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Invalid query parameter: {}", e));
        }
    };

    // Convert the JSON payload to a MongoDB document
    let update_doc = match mongodb::bson::to_document(&payload) {
        Ok(doc) => doc,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Invalid document format: {}", e));
        }
    };

    // Create the update document with $set operator
    let update = doc! { "$set": update_doc };

    // Update the document(s)
    match data
        .db_client
        .database(&data.config.database_name)
        .collection::<mongodb::bson::Document>(&coll_name)
        .update_many(filter, update)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => {
            println!("Error updating document: {:?}", e);
            HttpResponse::InternalServerError().body(format!("Error updating document: {:?}", e))
        }
    }
}

#[delete("/{coll_name}")]
async fn delete_document(
    path: web::Path<String>,
    query: web::Query<std::collections::HashMap<String, String>>,
    data: web::Data<AppState>,
) -> impl Responder {
    let coll_name = path.into_inner();

    if let Some(e) = get_exception_if_collection_absent(coll_name.as_str(), &data).await {
        return e;
    }

    // Parse query parameters for filter
    let filter = match parse_match_query_params(&query) {
        Ok(filter) => filter,
        Err(e) => {
            return HttpResponse::BadRequest().body(format!("Invalid query parameter: {}", e));
        }
    };

    // Delete the document(s)
    match data
        .db_client
        .database(&data.config.database_name)
        .collection::<mongodb::bson::Document>(&coll_name)
        .delete_many(filter)
        .await
    {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(e) => {
            println!("Error deleting document: {:?}", e);
            HttpResponse::InternalServerError().body(format!("Error deleting document: {:?}", e))
        }
    }
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api").service(hello).service(ping))
        .service(query_collection)
        .service(insert_document)
        .service(update_document)
        .service(patch_document)
        .service(delete_document);
}

async fn get_exception_if_collection_absent(
    collection_name: &str,
    data: &web::Data<AppState>,
) -> Option<HttpResponse> {
    match crate::catalog::get_cached_collections(&data) {
        Some(catalog) => match catalog
            .collection_specs
            .iter()
            .find(|c| c.name == collection_name)
        {
            Some(_) => None,
            None => {
                match data
                    .db_client
                    .database(&data.config.database_name)
                    .list_collection_names()
                    .await
                {
                    Ok(names) => {
                        if names.contains(&collection_name.to_string()) {
                            None
                        } else {
                            Some(HttpResponse::NotFound().body(format!(
                                "Collection {} not found",
                                collection_name.to_owned()
                            )))
                        }
                    }
                    Err(_) => Some(
                        HttpResponse::InternalServerError()
                            .body("Failed to check collection existence"),
                    ),
                }
            }
        },
        None => {
            Some(HttpResponse::InternalServerError().body("Could not access collections catalog"))
        }
    }
}
