# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
cargo build

# Run (requires .env)
cargo run

# Check (fast, no binary)
cargo check

# Run tests
cargo test

# Run a single test
cargo test <test_name>

# Apply DB migrations
sqlx migrate run

# Add a new migration
sqlx migrate add <migration_name>
```

## Environment Variables (.env)

All required at startup — the server panics if any are missing:

```
PORT=
DATABASE_URL=
ACCESS_SECRET=
REFRESH_SECRET=
ACCESS_MINUTES=
REFRESH_DAYS=
```

## Architecture

The server is a JWT auth API built on axum + sqlx (PostgreSQL).

**Request flow:**
1. `main.rs` reads `PORT`, calls `create_app()`, binds TCP listener
2. `app.rs` reads `DATABASE_URL`, creates `PgPool`, builds the axum `Router` via `api::route()`
3. `AppState { pool }` is shared across all handlers via axum's `State` extractor

**Auth flow:**
- `POST /api/auth/authorize` — accepts `client_id`/`client_secret` (currently hardcoded `foo`/`bar`), issues access + refresh tokens
- `POST /api/auth/refresh` — `RefreshClaims` extractor validates the Bearer refresh token, then the handler checks `jti` against the `refresh_tokens` DB table
- `GET /api/auth/protected` — `AccessClaims` extractor validates the Bearer access token

**Token extraction pattern:** `AccessClaims` and `RefreshClaims` implement `FromRequestParts`, so they can be used directly as handler parameters. Both extract `Authorization: Bearer <token>` via `axum-extra`'s `TypedHeader`.

**ID generation (`id.rs`):** A single `LazyLock<Mutex<Generator>>` prevents ULID monotonicity collisions under concurrent async load. The `define_ids!` macro generates prefixed ID functions (e.g. `user_id()` → `"user-<ULID>"`) with compile-time checks for max 5-char prefix and no duplicates.

**JWT secrets** are loaded once into a `LazyLock<Keys>` at first use; `TOKEN_LIFE` is loaded similarly.

## Known Incomplete Areas

- `authorize`: credentials are hardcoded (`foo`/`bar`); no DB user lookup
- `refresh`: old refresh token is not deleted (token rotation not implemented)
- `users` handler: returns a dummy query result
- `refresh_tokens.jti` is `VARCHAR(26)` but prefixed IDs like `"token-<ULID>"` are longer than 26 chars
