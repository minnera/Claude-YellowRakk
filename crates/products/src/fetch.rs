//! Downloads shop pages.

use std::time::Duration;

use url::Url;

/// Pages larger than this are cut off; product metadata lives near the top.
const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("shop answered with HTTP {0}")]
    Status(reqwest::StatusCode),
}

/// A downloaded page, with the URL it ended up at after redirects.
#[derive(Debug, Clone)]
pub struct FetchedPage {
    pub final_url: Url,
    pub html: String,
}

#[derive(Debug, Clone)]
pub struct PageFetcher {
    client: reqwest::Client,
}

impl PageFetcher {
    pub fn new() -> Result<Self, FetchError> {
        // Many shops serve an empty shell or a captcha to unknown clients,
        // so present ourselves as a regular desktop browser.
        let client = reqwest::Client::builder()
            .user_agent(
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
                 (KHTML, like Gecko) Chrome/130.0 Safari/537.36",
            )
            .timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?;
        Ok(Self { client })
    }

    pub async fn fetch(&self, url: &Url) -> Result<FetchedPage, FetchError> {
        let mut response = self
            .client
            .get(url.clone())
            .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(FetchError::Status(response.status()));
        }
        let final_url = response.url().clone();

        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            let room = MAX_BODY_BYTES.saturating_sub(body.len());
            body.extend_from_slice(chunk.get(..room.min(chunk.len())).unwrap_or_default());
            if body.len() >= MAX_BODY_BYTES {
                tracing::debug!(%final_url, "page truncated at {MAX_BODY_BYTES} bytes");
                break;
            }
        }
        Ok(FetchedPage {
            final_url,
            html: String::from_utf8_lossy(&body).into_owned(),
        })
    }
}
