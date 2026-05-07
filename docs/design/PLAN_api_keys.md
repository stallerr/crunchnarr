# Plan: API Key Management

Add Sonarr-style API keys so users can authenticate requests with `X-Api-Key: <key>` instead of a JWT Bearer token. Keys are per-user, named, deletable, and the full key is shown only once at creation.

---

## 1. DB Migration — `005_api_keys.sql`

Create `crunchy-cli/crates/api/src/db/migrations/005_api_keys.sql`:

```sql
CREATE TABLE IF NOT EXISTS api_keys (
    id           TEXT PRIMARY KEY,
    user_id      TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    key_hash     TEXT NOT NULL UNIQUE,
    key_prefix   TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    last_used_at TEXT
);
```

`key_hash` = SHA-256 of the raw key (never store plaintext).
`key_prefix` = first 8 chars of the key after the `crl_` prefix, used for display only.

---

## 2. DB layer — `src/db/api_keys.rs`

Create `crunchy-cli/crates/api/src/db/api_keys.rs`:

```rust
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKeyRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub key_hash: String,
    pub key_prefix: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

pub async fn insert_api_key(pool, id, user_id, name, key_hash, key_prefix) -> Result<(), sqlx::Error>
pub async fn list_api_keys(pool, user_id) -> Result<Vec<ApiKeyRow>, sqlx::Error>
pub async fn get_api_key_by_hash(pool, key_hash) -> Result<Option<ApiKeyRow>, sqlx::Error>
pub async fn delete_api_key(pool, id, user_id) -> Result<bool, sqlx::Error>  // false = not found
pub async fn touch_api_key(pool, id) -> Result<(), sqlx::Error>              // updates last_used_at
```

Add `pub mod api_keys;` to `src/db/mod.rs`.

Also register the migration in `run_migrations()` — the project loads each one explicitly via `include_str!`, it does not auto-discover the migrations directory:

```rust
let migration_005 = include_str!("migrations/005_api_keys.sql");
sqlx::raw_sql(migration_005).execute(pool).await?;
```

No `if let Err(e) … "duplicate column"` wrapper is needed — that pattern exists for non-idempotent `ALTER TABLE` (003/004); 005 is pure `CREATE TABLE IF NOT EXISTS` and is naturally idempotent.

---

## 3. Key generation helpers

Place these in **`src/auth/api_key.rs`** (new file) — matches the existing split where `auth/jwt.rs` holds crypto and `db/users.rs` / `db/api_keys.rs` hold row types. Add `pub mod api_key;` to `src/auth/mod.rs`.

```rust
/// Generates a new API key: "crl_" + 64 hex chars (32 random bytes).
pub fn generate_api_key() -> String {
    use ring::rand::{SecureRandom, SystemRandom};
    let rng = SystemRandom::new();
    let mut bytes = [0u8; 32];
    rng.fill(&mut bytes).expect("rng failed");
    format!("crl_{}", hex::encode(bytes))
}

/// SHA-256 hash of the key, hex-encoded. Used for DB storage and lookup.
pub fn hash_api_key(key: &str) -> String {
    use ring::digest;
    let digest = digest::digest(&digest::SHA256, key.as_bytes());
    hex::encode(digest.as_ref())
}
```

Both `ring` (`ring = "0.17"`) and `hex` (`hex = "0.4"`) are already deps in `crates/api`.

**Why SHA-256 and not Argon2?** Passwords use Argon2 because they're low-entropy and need KDF stretching to resist offline brute-force. API keys carry 256 bits of entropy from a CSPRNG — stretching adds nothing, and hashing runs on the request hot path, so a fast cryptographic hash is the right tool.

---

## 4. Auth middleware — `src/auth/middleware.rs`

Extend `FromRequestParts<AppState> for AuthUser` to try both auth methods:

```
1. If "Authorization: Bearer <token>" header is present → existing JWT path (keep as-is)
2. Else if "X-Api-Key: <key>" header is present:
   a. Hash the key: hash_api_key(&key)
   b. Call db::api_keys::get_api_key_by_hash(&state.db, &hash).await
   c. If found → clone the pool and fire-and-forget the touch:
        let pool = state.db.clone();
        let id = row.id.clone();
        tokio::spawn(async move { let _ = db::api_keys::touch_api_key(&pool, &id).await; });
   d. Return AuthUser { user_id: row.user_id }
   e. If not found → return 401
3. If neither header → return 401
```

The pool clone is required: the `'static` bound on `tokio::spawn` won't accept a borrow of `state`.

This means **every existing protected route automatically accepts API keys** — no per-route changes needed.

---

## 5. Route handlers — `src/routes/api_keys.rs`

Create `crunchy-cli/crates/api/src/routes/api_keys.rs` with a `pub fn router() -> Router<AppState>`.

All three routes require `AuthUser` extractor.

### `GET /api-keys`
Returns the current user's keys. Never include `key_hash` in the response.

Response body:
```json
[
  {
    "id": "uuid",
    "name": "Home Assistant",
    "key_prefix": "a1b2c3d4",
    "created_at": "2026-05-04T...",
    "last_used_at": null
  }
]
```

### `POST /api-keys`
Request body:
```json
{ "name": "Home Assistant" }
```

- Validate: name non-empty, max 100 chars
- Generate key with `generate_api_key()`
- Extract `key_prefix` = `&key[4..12]` (8 chars after `crl_`)
- Hash with `hash_api_key(&key)`
- Insert into DB with a new `uuid::Uuid::new_v4().to_string()` as id
- Return **201** with:

```json
{
  "id": "uuid",
  "name": "Home Assistant",
  "key": "crl_a1b2c3d4...",
  "key_prefix": "a1b2c3d4",
  "created_at": "2026-05-04T..."
}
```

The `key` field is **returned once only** — it is not stored in the DB.

### `DELETE /api-keys/:id`
- Call `delete_api_key(&pool, &id, &auth_user.user_id)` (user_id scoping prevents deleting another user's key)
- If the call returns `Ok(false)` → map to `ApiError::NotFound`; on `Ok(true)` → return **204**

### Route security annotations

Annotate each handler with both auth schemes so the OpenAPI spec reflects that either works:

```rust
security(("bearer_auth" = []), ("api_key" = []))
```

Existing routes can keep `bearer_auth` only — they'll accept API keys at runtime regardless; the annotation is purely documentation.

### Wire into router

In `src/routes/mod.rs`:
```rust
pub mod api_keys;
```

In `build_router`:
```rust
.merge(api_keys::router())
```

---

## 6. utoipa docs — `src/docs.rs`

Register the three handler functions in `#[derive(OpenApi)]`'s `paths(...)` list.

Add to `components(schemas(...))`:
- `ApiKeyItem` (list response item)
- `CreateApiKeyRequest`
- `CreateApiKeyResponse`

Extend `SecurityAddon::modify` to also register the `api_key` scheme alongside `bearer_auth`:

```rust
use utoipa::openapi::security::{ApiKey, ApiKeyValue};

components.add_security_scheme(
    "api_key",
    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-Api-Key"))),
);
```

Add a new tag for the routes:

```rust
(name = "API Keys", description = "Per-user API keys for non-interactive auth"),
```

---

## 7. Frontend — settings page

In `crunchy-web/app/(protected)/settings/page.tsx`, add an **API Keys** section:

- Table columns: Name | Prefix | Created | Last Used | Revoke
- **"Create API Key"** button → modal with a name text input
  - On submit: `POST /api-keys`
  - On success: replace modal with a one-time reveal dialog showing the full `crl_...` key, a copy button, and the warning *"Store this key — you won't be able to see it again."*
- **Revoke** button → confirmation dialog → `DELETE /api-keys/:id` → remove row from table

---

## Validation checklist

After implementation:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo check -p crunchy-api --manifest-path crunchy-cli/Cargo.toml
cargo test --manifest-path crunchy-cli/Cargo.toml
cd crunchy-web && npm run build && npm run lint
```

Manual smoke test:
1. Create a key in the UI → copy the `crl_...` value
2. `curl -H "X-Api-Key: crl_..." http://localhost:8080/downloads` → returns data
3. `curl -H "Authorization: Bearer <jwt>" http://localhost:8080/downloads` → still works
4. Delete the key in the UI → repeat step 2 → should return 401
5. `last_used_at` on the key row should update after step 2

---

## Design notes

- Full key is **never stored** — only its SHA-256 hash. Lost key = create a new one.
- API key and JWT auth work on **all protected routes** — the middleware handles both transparently.
- A key always resolves to the user who created it, with identical permissions.
- `last_used_at` is updated via fire-and-forget `tokio::spawn` to avoid adding latency to the request.
- Migration idempotency: follow the same `IF NOT EXISTS` pattern used in earlier migrations.
- `ON DELETE CASCADE` is used on `api_keys.user_id` (existing tables don't cascade) — orphan API keys would be a security hazard, so the new convention is intentional.
- `key_hash UNIQUE` doubles as the lookup index — no separate `CREATE INDEX` needed.
