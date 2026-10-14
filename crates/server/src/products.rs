//! `/api/products` routes.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use yellowrakk_products::{ProductDetail, ProductId, ProductSummary};

use crate::{ApiError, AppState};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(import))
        .route("/{id}", get(detail).delete(remove))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImportRequest {
    url: String,
}

async fn import(
    State(state): State<AppState>,
    Json(req): Json<ImportRequest>,
) -> Result<(StatusCode, Json<ProductDetail>), ApiError> {
    let product = state.products.import(&req.url).await?;
    Ok((StatusCode::CREATED, Json(product)))
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<ProductSummary>>, ApiError> {
    Ok(Json(state.products.list().await?))
}

async fn detail(
    State(state): State<AppState>,
    Path(id): Path<ProductId>,
) -> Result<Json<ProductDetail>, ApiError> {
    state
        .products
        .get(id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

async fn remove(
    State(state): State<AppState>,
    Path(id): Path<ProductId>,
) -> Result<StatusCode, ApiError> {
    if state.products.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
