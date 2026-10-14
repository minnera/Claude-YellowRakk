//! Shared building blocks used by every YellowRakk domain crate.

pub mod id;
pub mod money;

pub use money::{Currency, Money, MoneyError};
