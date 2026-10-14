//! Database access for products. All queries are checked against the
//! schema at compile time by `sqlx::query!`.

use sqlx::SqlitePool;
use time::OffsetDateTime;
use yellowrakk_core::{Currency, Money};

use crate::model::{Listing, ListingId, ListingStatus, ProductDetail, ProductId, ProductSummary};

/// Data for a brand-new product created from a pasted link.
#[derive(Debug, Clone)]
pub struct NewProduct<'a> {
    pub name: &'a str,
    pub url: &'a str,
    pub shop: &'a str,
    pub image_url: Option<&'a str>,
    pub price: Option<Money>,
}

#[derive(Debug, Clone)]
pub struct ProductRepo {
    pool: SqlitePool,
}

impl ProductRepo {
    #[must_use]
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Returns the product that already has a listing with this URL.
    pub async fn find_by_listing_url(&self, url: &str) -> sqlx::Result<Option<ProductId>> {
        sqlx::query_scalar!(
            r#"SELECT product_id AS "product_id: ProductId" FROM listings WHERE url = ?"#,
            url
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Stores a product together with its source listing and first price.
    pub async fn create(&self, new: NewProduct<'_>) -> sqlx::Result<ProductId> {
        let now = OffsetDateTime::now_utc();
        let product_id = ProductId::new();
        let listing_id = ListingId::new();
        let status = ListingStatus::Source;

        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "INSERT INTO products (id, name, created_at, updated_at) VALUES (?, ?, ?, ?)",
            product_id,
            new.name,
            now,
            now,
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query!(
            "INSERT INTO listings (id, product_id, url, shop, title, image_url, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            listing_id,
            product_id,
            new.url,
            new.shop,
            new.name,
            new.image_url,
            status,
            now,
        )
        .execute(&mut *tx)
        .await?;
        if let Some(price) = new.price {
            let currency = price.currency.as_str();
            sqlx::query!(
                "INSERT INTO price_observations (listing_id, minor_units, currency, observed_at)
                 VALUES (?, ?, ?, ?)",
                listing_id,
                price.minor_units,
                currency,
                now,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(product_id)
    }

    pub async fn list(&self) -> sqlx::Result<Vec<ProductSummary>> {
        sqlx::query_as!(
            ProductSummary,
            r#"SELECT
                 p.id AS "id: ProductId",
                 p.name,
                 (SELECT l.image_url FROM listings l
                   WHERE l.product_id = p.id AND l.status = 'source') AS "image_url?: String",
                 (SELECT count(*) FROM listings l
                   WHERE l.product_id = p.id AND l.status != 'rejected') AS "listing_count!: i64",
                 p.created_at AS "created_at: OffsetDateTime"
               FROM products p
               ORDER BY p.created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get(&self, id: ProductId) -> sqlx::Result<Option<ProductDetail>> {
        let Some(product) = sqlx::query!(
            r#"SELECT
                 name,
                 notes,
                 created_at AS "created_at: OffsetDateTime",
                 updated_at AS "updated_at: OffsetDateTime"
               FROM products WHERE id = ?"#,
            id
        )
        .fetch_optional(&self.pool)
        .await?
        else {
            return Ok(None);
        };

        let rows = sqlx::query!(
            r#"SELECT
                 l.id AS "id: ListingId",
                 l.url,
                 l.shop,
                 l.title,
                 l.image_url,
                 l.status AS "status: ListingStatus",
                 l.created_at AS "created_at: OffsetDateTime",
                 p.minor_units AS "minor_units?: i64",
                 p.currency AS "currency?: String"
               FROM listings l
               LEFT JOIN price_observations p ON p.id = (
                 SELECT id FROM price_observations
                 WHERE listing_id = l.id
                 ORDER BY observed_at DESC, id DESC
                 LIMIT 1
               )
               WHERE l.product_id = ?
               ORDER BY l.created_at"#,
            id
        )
        .fetch_all(&self.pool)
        .await?;

        let listings = rows
            .into_iter()
            .map(|row| {
                let latest_price = match (row.minor_units, row.currency) {
                    (Some(minor_units), Some(code)) => {
                        let currency: Currency =
                            code.parse().map_err(|e| sqlx::Error::Decode(Box::new(e)))?;
                        Some(Money {
                            minor_units,
                            currency,
                        })
                    }
                    _ => None,
                };
                Ok(Listing {
                    id: row.id,
                    url: row.url,
                    shop: row.shop,
                    title: row.title,
                    image_url: row.image_url,
                    status: row.status,
                    latest_price,
                    created_at: row.created_at,
                })
            })
            .collect::<sqlx::Result<Vec<_>>>()?;

        Ok(Some(ProductDetail {
            id,
            name: product.name,
            notes: product.notes,
            listings,
            created_at: product.created_at,
            updated_at: product.updated_at,
        }))
    }

    /// Deletes a product and, via cascade, its listings and prices.
    /// Returns `false` if it did not exist.
    pub async fn delete(&self, id: ProductId) -> sqlx::Result<bool> {
        let result = sqlx::query!("DELETE FROM products WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
