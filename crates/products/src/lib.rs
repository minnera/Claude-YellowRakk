//! Purchasable products.
//!
//! Flow: a pasted shop link is normalised ([`link`]), fetched ([`fetch`]),
//! turned into structured data ([`metadata`]) and stored ([`repo`]).
//! [`service::ProductService`] wires these together.

pub mod fetch;
pub mod link;
pub mod metadata;
pub mod model;
pub mod repo;
pub mod service;

pub use model::{Listing, ListingId, ListingStatus, ProductDetail, ProductId, ProductSummary};
pub use service::{ImportError, ProductService};
