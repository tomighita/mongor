use actix_web::{App, HttpServer, rt::System, web};
use mongodb::{Client, options::ClientOptions};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod catalog;
mod config;
mod routes;

pub mod shared {
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    pub struct AppState {
        pub config: crate::config::AppConfig,
        pub db_client: mongodb::Client,
        pub collections: Arc<Mutex<crate::catalog::Catalog>>,
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = config::load_config();

    let options = ClientOptions::parse(&config.database_conn_url)
        .await
        .expect("failed to parse config");
    let db_client = Client::with_options(options).expect("failed to create client");

    println!("Successfully connected to MongoDB!");

    let init_catalog = catalog::fetch_all_collections(&db_client.database(&config.database_name))
        .await
        .expect("Error fetching initial catalog");

    // Create the shared state
    let app_state = web::Data::new(crate::shared::AppState {
        config: config.clone(),
        db_client: db_client.clone(),
        collections: Arc::new(Mutex::new(init_catalog)),
    });

    // Spawn a background task to periodically fetch catalog
    actix_web::rt::spawn(catalog::fetch_collections_periodically(
        app_state.clone(),
        Duration::from_secs(5),
    ));

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .configure(routes::configure)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
