//! Strongly typed identifiers.
//!
//! Every entity gets its own ID type so a `ProductId` can never be passed
//! where a `ListingId` is expected. Declare new ones with [`typed_id!`].

/// Declares a UUIDv7-backed ID newtype that works with serde and sqlx.
///
/// ```
/// yellowrakk_core::typed_id!(
///     /// Identifies a widget.
///     WidgetId
/// );
/// let id = WidgetId::new();
/// assert_eq!(id, WidgetId::from_uuid(id.into_uuid()));
/// ```
#[macro_export]
macro_rules! typed_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
            ::serde::Serialize, ::serde::Deserialize, ::sqlx::Type,
        )]
        #[serde(transparent)]
        #[sqlx(transparent)]
        pub struct $name(::uuid::Uuid);

        impl $name {
            /// Creates a new, time-ordered ID.
            #[must_use]
            pub fn new() -> Self {
                Self(::uuid::Uuid::now_v7())
            }

            #[must_use]
            pub const fn from_uuid(uuid: ::uuid::Uuid) -> Self {
                Self(uuid)
            }

            #[must_use]
            pub const fn into_uuid(self) -> ::uuid::Uuid {
                self.0
            }
        }

        impl ::core::default::Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl ::core::fmt::Display for $name {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                ::core::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}
