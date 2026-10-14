//! Domain types for the products area.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use yellowrakk_core::Money;

yellowrakk_core::typed_id!(
    /// Identifies a product (the thing itself, independent of shop).
    ProductId
);
yellowrakk_core::typed_id!(
    /// Identifies one shop listing of a product.
    ListingId
);

/// How a listing relates to its product.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(rename_all = "lowercase")]
pub enum ListingStatus {
    /// The link the user pasted.
    Source,
    /// Found by search, waiting for review.
    Candidate,
    /// Verified to be the same product.
    Confirmed,
    /// Verified to be a different product.
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProductSummary {
    pub id: ProductId,
    pub name: String,
    pub image_url: Option<String>,
    pub listing_count: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProductDetail {
    pub id: ProductId,
    pub name: String,
    pub notes: Option<String>,
    pub listings: Vec<Listing>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Listing {
    pub id: ListingId,
    pub url: String,
    pub shop: String,
    pub title: String,
    pub image_url: Option<String>,
    pub status: ListingStatus,
    pub latest_price: Option<Money>,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}
