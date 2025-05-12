use actix_web::{HttpResponse, Responder, get, web};
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

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api").service(hello).service(ping));
}
