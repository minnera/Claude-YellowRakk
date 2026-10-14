# YellowRakk

A personal, self-hosted database for things I buy, wear, eat and cook.

| Area | What it does | Status |
|------|--------------|--------|
| **Products** | Paste a shop link (Amazon, Temu, anything) → saved with title, image, price. Later: finds the same item in other shops and tracks prices. | Backend v0 |
| **Wardrobe** | Clothes, outfits, recommendations based on taste. | Planned |
| **Dining** | Rate restaurants, dishes and drinks. | Planned |
| **Recipes** | Recipes with multiple dated *cooks* each: what changed, how it turned out, photos, comments. | Planned |

## Architecture

```
crates/
  core/       shared types: typed IDs, Money (integer minor units, never floats)
  products/   link normalising, page fetching, metadata extraction, DB access
  server/     Axum HTTP API, thin handlers only
migrations/   SQLite schema (sqlx migrations)
.sqlx/        compile-time query metadata, lets CI build without a database
```

Each area is its own crate. Adding one means adding a crate and a migration;
the others don't change.

**Guard rails**
- Every SQL query is checked against the real schema at compile time (`sqlx::query!`).
- Every entity has its own ID type, so a `ListingId` can't be passed where a `ProductId` is expected.
- `unsafe` is forbidden; clippy pedantic plus `unwrap`/`panic`/`as`-cast bans; CI treats warnings as errors.
- Money is `i64` minor units plus an ISO currency code.

### How product import works

1. The link is normalised: tracking params are dropped, and Amazon links collapse to `/dp/<ASIN>`.
2. The page is fetched. Data comes from schema.org JSON-LD first, then Open Graph tags, then `<title>`.
3. A **product** (the thing) is stored with its first **listing** (where it's sold) and a **price observation**.
   Look-alikes found later become extra listings (`candidate` → `confirmed` / `rejected`).

Some shops (Temu in particular) block non-browser clients. Those links
will fail for now; screenshot-based extraction with Claude is on the roadmap.

## API

| Method | Path | |
|--------|------|-|
| `POST` | `/api/products` | `{"url": "..."}` imports a link. Returns `409` with `existing_product_id` if it's already saved. |
| `GET` | `/api/products` | List |
| `GET` | `/api/products/{id}` | Detail with listings and latest prices |
| `DELETE` | `/api/products/{id}` | Delete |
| `GET` | `/health` | Liveness |

## Running

**Docker (home server / NAS):**
```sh
docker compose up -d --build
```
It listens on `127.0.0.1:8080` because **there is no login yet**. To use it
from your phone, install [Tailscale](https://tailscale.com) on both devices
and run `tailscale serve 8080` on the server instead of exposing the port.

**Development:**
```sh
cargo install sqlx-cli --no-default-features --features sqlite
cp .env.example .env            # DATABASE_URL etc.
sqlx database create && sqlx migrate run
cargo run
```

After changing a query or migration, run `cargo sqlx prepare --workspace`
and commit `.sqlx/`. CI builds offline and fails if it's stale.

```sh
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

## Roadmap

1. **PWA frontend** (Leptos, Rust in the browser, sharing types with the API). Includes an Android share target, so "Share → YellowRakk" from any shop app saves the link.
2. **Similar-product search** behind a swappable `SimilarProductFinder` trait. Candidate providers: SerpApi Google Shopping/Lens or Claude web search (cost decision pending).
3. **Claude Vision duplicate check** that compares listing images and proposes `confirmed`/`rejected`.
4. **Fallback extraction** for blocked shops via headless browser screenshot plus Claude.
5. **Price tracking** with periodic re-fetch (the schema already keeps history).
6. Auth, then Wardrobe, Dining and Recipes crates.
