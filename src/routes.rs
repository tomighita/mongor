use actix_web::{HttpResponse, Responder, get, web};
use futures_util::TryStreamExt;
use mongodb::bson::doc;

use crate::shared::AppState;

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

    // Parse query parameters
    let mut filter = doc! {};

    for (key, _) in query.iter() {
        if key.starts_with("match:") {
            let parts: Vec<&str> = key.splitn(3, ':').collect();
            if parts.len() == 3 {
                let field_name = parts[1];
                // Use the value from the URL parameter key instead of the empty value
                let field_value = parts[2];

                // Try to parse the value as a number if possible
                if let Ok(num) = field_value.parse::<i32>() {
                    filter.insert(field_name, num);
                } else {
                    filter.insert(field_name, field_value.to_string());
                }
            }
        } else {
            // Fail if the query parameter is not a match
            return HttpResponse::BadRequest().body("Invalid query parameter");
        }
    }

    // Execute the query
    match data
        .db_client
        .database(&data.config.database_name)
        .collection::<mongodb::bson::Document>(&coll_name)
        .find(filter)
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

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api").service(hello).service(ping))
        .service(query_collection);
}
