# YellowRakk: notes for Claude

Personal self-hosted app. Rust workspace: `crates/core`, `crates/products`, `crates/server`.
UI language is English. The owner talks in Hungarian.

## Rules
- Before every commit: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
- Use `sqlx::query!`/`query_as!` macros only, never the unchecked string APIs. After touching SQL or
  migrations run `cargo sqlx prepare --workspace` (needs `DATABASE_URL`, see `.env.example`) and commit `.sqlx/`.
- Migrations are append-only. Never edit an applied one; add a new file. Tables use `STRICT`.
- New entity → new ID via `yellowrakk_core::typed_id!`. Money → `yellowrakk_core::Money`.
- A new area (wardrobe, dining, recipes) gets its own crate. Business logic lives in the domain crate
  (`service.rs`); `server` handlers only translate HTTP ↔ service calls.
- No `unwrap`/`expect`/`panic`/`as` casts outside tests. Return typed `thiserror` errors.
- Each module explains its "why" in a `//!` doc comment. Keep those up to date.
