//! End-to-end API tests: an in-memory database and a fake shop on localhost.
#![allow(clippy::unwrap_used, reason = "test helpers should fail loudly")]

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::response::Html;
use axum::routing::get;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;
use yellowrakk_products::fetch::PageFetcher;
use yellowrakk_server::{AppState, router};

const SHOP_PAGE: &str = r#"<html><head>
  <title>Fake Shop</title>
  <script type="application/ld+json">
    {"@type":"Product","name":"Yellow Rakk Plushie","image":"/rakk.jpg",
     "offers":{"price":"12.99","priceCurrency":"EUR"}}
  </script>
</head></html>"#;

/// Starts a fake shop and returns its base URL.
async fn fake_shop() -> String {
    let app = Router::new().route("/item", get(|| async { Html(SHOP_PAGE) }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}")
}

async fn app() -> Router {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::migrate!("../../migrations").run(&pool).await.unwrap();
    router(AppState::new(pool, PageFetcher::new().unwrap()))
}

async fn call(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn import_list_get_delete() {
    let shop = fake_shop().await;
    let app = app().await;
    let link = format!("{shop}/item?utm_source=newsletter");

    let (status, created) = call(&app, "POST", "/api/products", Some(json!({ "url": link }))).await;
    assert_eq!(status, StatusCode::CREATED, "{created}");
    assert_eq!(created["name"], "Yellow Rakk Plushie");
    let listing = &created["listings"][0];
    assert_eq!(listing["url"], format!("{shop}/item"));
    assert_eq!(listing["status"], "source");
    assert_eq!(listing["image_url"], format!("{shop}/rakk.jpg"));
    assert_eq!(
        listing["latest_price"],
        json!({ "minor_units": 1299, "currency": "EUR" })
    );
    let id = created["id"].as_str().unwrap();

    let (status, dup) = call(&app, "POST", "/api/products", Some(json!({ "url": link }))).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(dup["existing_product_id"], id);

    let (status, list) = call(&app, "GET", "/api/products", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(list.as_array().unwrap().len(), 1);
    assert_eq!(list[0]["listing_count"], 1);

    let (status, detail) = call(&app, "GET", &format!("/api/products/{id}"), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(detail, created);

    let (status, _) = call(&app, "DELETE", &format!("/api/products/{id}"), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = call(&app, "GET", &format!("/api/products/{id}"), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn rejects_bad_links() {
    let app = app().await;
    let (status, body) = call(
        &app,
        "POST",
        "/api/products",
        Some(json!({ "url": "ftp://x.example/a" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body["error"].as_str().unwrap().contains("http"));
}
