//! Maps domain errors to HTTP responses with a JSON body `{ "error": "..." }`.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use yellowrakk_products::{ImportError, ProductId};

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("not found")]
    NotFound,
    #[error(transparent)]
    Import(#[from] ImportError),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Serialize)]
struct Body {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    existing_product_id: Option<ProductId>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut existing_product_id = None;
        let status = match &self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Import(e) => match e {
                ImportError::InvalidLink(_) | ImportError::NoTitle => {
                    StatusCode::UNPROCESSABLE_ENTITY
                }
                ImportError::AlreadySaved(id) => {
                    existing_product_id = Some(*id);
                    StatusCode::CONFLICT
                }
                ImportError::Fetch(_) => StatusCode::BAD_GATEWAY,
                ImportError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let error = if status.is_server_error() && status != StatusCode::BAD_GATEWAY {
            tracing::error!(error = %self, "request failed");
            "internal error".to_owned()
        } else {
            self.to_string()
        };
        (
            status,
            Json(Body {
                error,
                existing_product_id,
            }),
        )
            .into_response()
    }
}
