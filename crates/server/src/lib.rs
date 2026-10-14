//! HTTP layer. Handlers stay thin: parse input, call a domain service,
//! map the result to JSON. Business rules belong in the domain crates.

mod error;
mod products;

use axum::Router;
use axum::routing::get;
use sqlx::SqlitePool;
use tower_http::trace::TraceLayer;
use yellowrakk_products::ProductService;
use yellowrakk_products::fetch::PageFetcher;

pub use error::ApiError;

#[derive(Debug, Clone)]
pub struct AppState {
    products: ProductService,
}

impl AppState {
    #[must_use]
    pub const fn new(pool: SqlitePool, fetcher: PageFetcher) -> Self {
        Self {
            products: ProductService::new(pool, fetcher),
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .nest("/api/products", products::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
