//! Product use-cases, independent of HTTP.

use sqlx::SqlitePool;

use crate::fetch::{FetchError, PageFetcher};
use crate::link::{LinkError, ShopLink};
use crate::metadata;
use crate::model::{ProductDetail, ProductId, ProductSummary};
use crate::repo::{NewProduct, ProductRepo};

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error(transparent)]
    InvalidLink(#[from] LinkError),
    #[error("this link is already saved")]
    AlreadySaved(ProductId),
    #[error("could not download the page: {0}")]
    Fetch(#[from] FetchError),
    #[error("the page has no recognisable product title")]
    NoTitle,
    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct ProductService {
    repo: ProductRepo,
    fetcher: PageFetcher,
}

impl ProductService {
    #[must_use]
    pub const fn new(pool: SqlitePool, fetcher: PageFetcher) -> Self {
        Self {
            repo: ProductRepo::new(pool),
            fetcher,
        }
    }

    /// Saves a product from a pasted shop link.
    pub async fn import(&self, raw_link: &str) -> Result<ProductDetail, ImportError> {
        let link = ShopLink::parse(raw_link)?;
        self.ensure_new(&link).await?;

        let page = self.fetcher.fetch(link.url()).await?;
        let meta = metadata::extract(&page.html, &page.final_url);

        // Prefer the shop's own canonical URL when it is on the same shop,
        // so a later paste of a different-looking link is still detected.
        let link = match meta.canonical_url.clone().map(ShopLink::from_url) {
            Some(Ok(canonical)) if canonical.shop() == link.shop() => {
                self.ensure_new(&canonical).await?;
                canonical
            }
            _ => link,
        };

        let name = meta.title.ok_or(ImportError::NoTitle)?;
        let image_url = meta.image_url.as_ref().map(url::Url::as_str);
        let created = self
            .repo
            .create(NewProduct {
                name: &name,
                url: link.url().as_str(),
                shop: link.shop(),
                image_url,
                price: meta.price,
            })
            .await;
        let id = match created {
            Ok(id) => id,
            // Lost a race with a concurrent import of the same link.
            Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
                self.ensure_new(&link).await?;
                return Err(ImportError::Database(sqlx::Error::Database(e)));
            }
            Err(e) => return Err(e.into()),
        };

        tracing::info!(%id, shop = link.shop(), "imported product");
        self.repo
            .get(id)
            .await?
            .ok_or(ImportError::Database(sqlx::Error::RowNotFound))
    }

    async fn ensure_new(&self, link: &ShopLink) -> Result<(), ImportError> {
        match self.repo.find_by_listing_url(link.url().as_str()).await? {
            Some(existing) => Err(ImportError::AlreadySaved(existing)),
            None => Ok(()),
        }
    }

    pub async fn list(&self) -> sqlx::Result<Vec<ProductSummary>> {
        self.repo.list().await
    }

    pub async fn get(&self, id: ProductId) -> sqlx::Result<Option<ProductDetail>> {
        self.repo.get(id).await
    }

    pub async fn delete(&self, id: ProductId) -> sqlx::Result<bool> {
        self.repo.delete(id).await
    }
}
