use actix_web::{App, HttpServer, web};
use mongodb::{Client, options::ClientOptions};

mod config;
mod routes;

pub mod shared {
    #[derive(Clone)]
    pub struct AppState {
        pub config: crate::config::AppConfig,
        pub db_client: mongodb::Client,
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = config::load_config();

    let options = ClientOptions::parse(&config.database_conn_url)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let db_client = Client::with_options(options)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    println!("Successfully connected to MongoDB!");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(crate::shared::AppState {
                config: config.clone(),
                db_client: db_client.clone(),
            }))
            .configure(routes::configure)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
