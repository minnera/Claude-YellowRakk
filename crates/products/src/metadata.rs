//! Extracts product data from a shop page.
//!
//! Sources, best first:
//! 1. schema.org `Product` JSON-LD (most shops ship it for Google),
//! 2. Open Graph / `product:` meta tags,
//! 3. the plain `<title>`.

use scraper::{Html, Selector};
use serde_json::Value;
use url::Url;
use yellowrakk_core::{Currency, Money};

/// Everything we could learn about a product from its page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageMetadata {
    pub title: Option<String>,
    pub image_url: Option<Url>,
    pub price: Option<Money>,
    pub canonical_url: Option<Url>,
}

#[must_use]
pub fn extract(html: &str, page_url: &Url) -> PageMetadata {
    let doc = Html::parse_document(html);
    let json_ld = json_ld_product(&doc);
    let meta = |names: &[&str]| meta_content(&doc, names);

    let title = json_ld
        .as_ref()
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| meta(&["og:title", "twitter:title"]))
        .or_else(|| first_text(&doc, "title"))
        .map(|t| collapse_whitespace(&t))
        .filter(|t| !t.is_empty());

    let image_url = json_ld
        .as_ref()
        .and_then(|p| p.get("image"))
        .and_then(first_image)
        .or_else(|| meta(&["og:image", "og:image:secure_url", "twitter:image"]))
        .and_then(|raw| page_url.join(&raw).ok());

    let price = json_ld.as_ref().and_then(json_ld_price).or_else(|| {
        let amount = meta(&["product:price:amount", "og:price:amount"])?;
        let currency = meta(&["product:price:currency", "og:price:currency"])?;
        Money::parse(&amount, currency.parse().ok()?).ok()
    });

    let canonical_url = canonical(&doc)
        .or_else(|| meta(&["og:url"]))
        .and_then(|raw| page_url.join(&raw).ok())
        .filter(|u| u.host_str() == page_url.host_str());

    PageMetadata {
        title,
        image_url,
        price,
        canonical_url,
    }
}

fn selector(css: &str) -> Option<Selector> {
    Selector::parse(css).ok()
}

fn meta_content(doc: &Html, names: &[&str]) -> Option<String> {
    let sel = selector("meta[content]")?;
    names.iter().find_map(|name| {
        doc.select(&sel).find_map(|el| {
            let attrs = el.value();
            let key = attrs.attr("property").or_else(|| attrs.attr("name"))?;
            let content = attrs.attr("content")?.trim();
            (key.eq_ignore_ascii_case(name) && !content.is_empty()).then(|| content.to_owned())
        })
    })
}

fn first_text(doc: &Html, css: &str) -> Option<String> {
    let sel = selector(css)?;
    doc.select(&sel).next().map(|el| el.text().collect())
}

fn canonical(doc: &Html) -> Option<String> {
    let sel = selector(r#"link[rel="canonical"][href]"#)?;
    doc.select(&sel)
        .next()?
        .value()
        .attr("href")
        .map(str::to_owned)
}

fn collapse_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Finds the first schema.org `Product` object in any JSON-LD block,
/// including ones nested in arrays or `@graph`.
fn json_ld_product(doc: &Html) -> Option<Value> {
    let sel = selector(r#"script[type="application/ld+json"]"#)?;
    doc.select(&sel)
        .filter_map(|el| serde_json::from_str::<Value>(&el.text().collect::<String>()).ok())
        .find_map(find_product)
}

fn find_product(value: Value) -> Option<Value> {
    match value {
        Value::Array(items) => items.into_iter().find_map(find_product),
        Value::Object(mut obj) => {
            if is_type(obj.get("@type"), "Product") {
                return Some(Value::Object(obj));
            }
            obj.remove("@graph").and_then(find_product)
        }
        _ => None,
    }
}

fn is_type(value: Option<&Value>, wanted: &str) -> bool {
    match value {
        Some(Value::String(s)) => s == wanted,
        Some(Value::Array(types)) => types.iter().any(|t| t.as_str() == Some(wanted)),
        _ => false,
    }
}

fn first_image(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Array(items) => items.iter().find_map(first_image),
        Value::Object(obj) => obj.get("url").and_then(Value::as_str).map(str::to_owned),
        _ => None,
    }
}

fn json_ld_price(product: &Value) -> Option<Money> {
    let offers = product.get("offers")?;
    let offer_list: Vec<&Value> = match offers {
        Value::Array(items) => items.iter().collect(),
        other => vec![other],
    };
    offer_list.into_iter().find_map(|offer| {
        let currency: Currency = offer.get("priceCurrency")?.as_str()?.parse().ok()?;
        let amount = offer
            .get("price")
            .or_else(|| offer.get("lowPrice"))
            .and_then(|p| match p {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })?;
        Money::parse(&amount, currency).ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Url {
        Url::parse("https://shop.example/items/42?x=1").unwrap()
    }

    #[test]
    fn prefers_json_ld_product() {
        let html = r#"<html><head>
            <title>Shop | Ignore me</title>
            <meta property="og:title" content="OG title">
            <script type="application/ld+json">
            {"@context":"https://schema.org","@graph":[
              {"@type":"BreadcrumbList"},
              {"@type":["Product","Thing"],"name":"  Yellow   Lamp ",
               "image":[{"url":"/img/lamp.jpg"}],
               "offers":[{"@type":"Offer","price":24.5,"priceCurrency":"eur"}]}
            ]}
            </script>
            <link rel="canonical" href="/items/42">
        </head></html>"#;
        let meta = extract(html, &page());
        assert_eq!(meta.title.as_deref(), Some("Yellow Lamp"));
        assert_eq!(
            meta.image_url.unwrap().as_str(),
            "https://shop.example/img/lamp.jpg"
        );
        assert_eq!(meta.price.unwrap().to_string(), "24.50 EUR");
        assert_eq!(
            meta.canonical_url.unwrap().as_str(),
            "https://shop.example/items/42"
        );
    }

    #[test]
    fn falls_back_to_open_graph() {
        let html = r#"<head>
            <meta property="og:title" content="OG Lamp">
            <meta property="og:image" content="https://cdn.example/lamp.png">
            <meta property="product:price:amount" content="1,299.00">
            <meta property="product:price:currency" content="USD">
        </head>"#;
        let meta = extract(html, &page());
        assert_eq!(meta.title.as_deref(), Some("OG Lamp"));
        assert_eq!(
            meta.image_url.unwrap().as_str(),
            "https://cdn.example/lamp.png"
        );
        assert_eq!(meta.price.unwrap().minor_units, 129_900);
    }

    #[test]
    fn falls_back_to_title_tag_and_ignores_foreign_canonical() {
        let html = r#"<head><title>
            Plain   Lamp </title>
            <meta property="og:url" content="https://evil.example/elsewhere">
        </head>"#;
        let meta = extract(html, &page());
        assert_eq!(meta.title.as_deref(), Some("Plain Lamp"));
        assert_eq!(meta.canonical_url, None);
        assert_eq!(meta.price, None);
    }

    #[test]
    fn empty_page_yields_nothing() {
        assert_eq!(extract("", &page()), PageMetadata::default());
    }
}
