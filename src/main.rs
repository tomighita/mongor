use actix_web::{App, HttpServer, rt::System, web};
use mongodb::{Client, options::ClientOptions};
use std::env;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use utoipa::{OpenApi, openapi};
use utoipa_swagger_ui::SwaggerUi;

mod catalog;
mod config;
mod openapi_docs;
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
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let port = args
        .iter()
        .position(|arg| arg == "--port")
        .and_then(|i| args.get(i + 1))
        .and_then(|port_str| port_str.parse::<u16>().ok())
        .unwrap_or(8080);

    let config = config::load_config();

    let options = ClientOptions::parse(&config.database_conn_url)
        .await
        .expect("failed to parse config");
    let db_client = Client::with_options(options).expect("failed to create client");

    println!("Successfully connected to MongoDB!");
    println!("Starting server on port {}", port);

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
        Duration::from_secs(60),
    ));

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            // Add Swagger UI with a dynamic path to the OpenAPI JSON
            .service(crate::openapi_docs::get_openapi_json)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/openapi.json", openapi_docs::ApiDoc::openapi()),
            )
            .configure(routes::configure)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
