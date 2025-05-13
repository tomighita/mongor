use actix_web::web;
use futures::TryStreamExt;
use mongodb::results::CollectionSpecification;
use std::time::Duration;

use crate::shared::AppState;

#[derive(Debug, Clone)]
pub struct Catalog {
    collection_specs: Vec<CollectionSpecification>,
}

/// Fetches all collections from the MongoDB database and their contents
pub async fn fetch_all_collections(
    database: &mongodb::Database,
) -> Result<Catalog, mongodb::error::Error> {
    let cursor = database.list_collections().await?;
    // Consume cursor
    let collections: Vec<_> = cursor.try_collect().await?;

    Ok(Catalog {
        collection_specs: collections,
    })
}

/// Runs in the background and periodically fetches MongoDB collections
pub async fn fetch_collections_periodically(state: web::Data<AppState>, interval: Duration) {
    let db = state.db_client.database(&state.config.database_name);
    loop {
        match fetch_all_collections(&db).await {
            Ok(catalog) => {
                // Update the shared state with the new collections
                if let Ok(mut locked_catalog) = state.collections.lock() {
                    *locked_catalog = catalog;
                    println!("Successfully updated catalog: {:?}.", locked_catalog);
                } else {
                    eprintln!("Failed to acquire lock on collections");
                }
            }
            Err(e) => {
                eprintln!("Error fetching collections: {}", e);
            }
        }

        // Sleep for the specified interval
        tokio::time::sleep(interval).await;
    }
}

/// Returns the current cached collections
pub fn get_cached_collections(state: &web::Data<AppState>) -> Option<Catalog> {
    state.collections.lock().ok().map(|guard| guard.clone())
}
