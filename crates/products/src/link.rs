//! Normalising pasted shop links so the same product is not stored twice
//! just because one link carried tracking parameters.

use url::Url;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LinkError {
    #[error("not a valid URL: {0}")]
    Parse(#[from] url::ParseError),
    #[error("only http and https links are supported")]
    UnsupportedScheme,
    #[error("link has no host")]
    MissingHost,
}

/// A validated, normalised product link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShopLink {
    url: Url,
    shop: String,
}

impl ShopLink {
    pub fn parse(input: &str) -> Result<Self, LinkError> {
        let url = Url::parse(input.trim())?;
        Self::from_url(url)
    }

    pub fn from_url(mut url: Url) -> Result<Self, LinkError> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(LinkError::UnsupportedScheme);
        }
        let host = url
            .host_str()
            .ok_or(LinkError::MissingHost)?
            .to_ascii_lowercase();
        let shop = host.strip_prefix("www.").unwrap_or(&host).to_owned();

        url.set_fragment(None);
        if let Some(asin) = amazon_asin(&shop, &url) {
            url.set_path(&format!("/dp/{asin}"));
            url.set_query(None);
        } else {
            strip_tracking_params(&mut url);
        }

        Ok(Self { url, shop })
    }

    #[must_use]
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// The shop's domain without `www.`, e.g. `amazon.de`.
    #[must_use]
    pub fn shop(&self) -> &str {
        &self.shop
    }
}

/// Amazon puts the product ID (ASIN) after `/dp/` or `/gp/product/`;
/// everything else in the URL is tracking noise.
fn amazon_asin(shop: &str, url: &Url) -> Option<String> {
    if !shop.split('.').any(|part| part == "amazon") {
        return None;
    }
    let segments: Vec<&str> = url.path_segments()?.collect();
    segments
        .windows(2)
        .find_map(|pair| match pair {
            ["dp" | "product", id] => Some(*id),
            _ => None,
        })
        .filter(|id| id.len() == 10 && id.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(str::to_ascii_uppercase)
}

fn strip_tracking_params(url: &mut Url) {
    const EXACT: &[&str] = &[
        "ref", "ref_", "gclid", "fbclid", "msclkid", "spm", "srsltid",
    ];
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| {
            let key = key.to_ascii_lowercase();
            !(key.starts_with("utm_") || EXACT.contains(&key.as_str()))
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if kept.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(kept);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amazon_links_collapse_to_asin() {
        let a = ShopLink::parse(
            "https://www.amazon.de/Some-Long-Name/dp/b0abc12345/ref=sr_1_3?keywords=x&qid=1",
        )
        .unwrap();
        let b = ShopLink::parse("https://amazon.de/gp/product/B0ABC12345?th=1").unwrap();
        assert_eq!(a.url().as_str(), "https://www.amazon.de/dp/B0ABC12345");
        assert_eq!(b.url().as_str(), "https://amazon.de/dp/B0ABC12345");
        assert_eq!(a.shop(), "amazon.de");
    }

    #[test]
    fn tracking_params_are_removed_but_others_kept() {
        let link = ShopLink::parse("https://shop.example/item?id=42&utm_source=x&gclid=y#reviews")
            .unwrap();
        assert_eq!(link.url().as_str(), "https://shop.example/item?id=42");
        assert_eq!(link.shop(), "shop.example");
    }

    #[test]
    fn rejects_non_web_links() {
        assert_eq!(
            ShopLink::parse("file:///etc/passwd"),
            Err(LinkError::UnsupportedScheme)
        );
        assert!(ShopLink::parse("not a url").is_err());
    }
}
