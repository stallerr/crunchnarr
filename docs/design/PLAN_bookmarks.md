# Plan: Bookmarked Shows

A simple per-user "saved series" list. Star icon toggles bookmark from the series page and from search-result cards; a new `/bookmarks` page shows the list with editable notes. No status workflow, no "+N new" badge — those are deferred.

---

## 1. DB Migration — `007_bookmarks.sql`

Create `crunchy-cli/crates/api/src/db/migrations/007_bookmarks.sql`:

```sql
CREATE TABLE IF NOT EXISTS bookmarks (
    user_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    series_id  TEXT NOT NULL,
    note       TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (user_id, series_id)
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_user_created
    ON bookmarks(user_id, created_at DESC);
```

Composite primary key `(user_id, series_id)` doubles as the uniqueness constraint and the lookup index. Register in `db/mod.rs::run_migrations`:

```rust
let migration_007 = include_str!("migrations/007_bookmarks.sql");
sqlx::raw_sql(migration_007).execute(pool).await?;
```

No `if let Err` wrapper needed — pure `CREATE TABLE IF NOT EXISTS`.

---

## 2. DB layer — `src/db/bookmarks.rs`

```rust
use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BookmarkRow {
    pub user_id: String,
    pub series_id: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn list_bookmarks(pool: &SqlitePool, user_id: &str)
    -> Result<Vec<BookmarkRow>, sqlx::Error>;

pub async fn upsert_bookmark(
    pool: &SqlitePool, user_id: &str, series_id: &str, note: &str,
    now: &str,
) -> Result<(), sqlx::Error>;
// Uses INSERT … ON CONFLICT DO UPDATE so create + edit-note are one path.
// updated_at always advances; created_at preserved on conflict.

pub async fn delete_bookmark(pool: &SqlitePool, user_id: &str, series_id: &str)
    -> Result<bool, sqlx::Error>;  // false = not found

pub async fn update_note(
    pool: &SqlitePool, user_id: &str, series_id: &str,
    note: &str, now: &str,
) -> Result<bool, sqlx::Error>;  // false = not found
```

Add `pub mod bookmarks;` to `src/db/mod.rs`.

---

## 3. Route handlers — `src/routes/bookmarks.rs`

```rust
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/bookmarks", get(list_bookmarks_route))
        .route("/bookmarks", post(create_bookmark))
        .route("/bookmarks/{series_id}", delete(delete_bookmark_route))
        .route("/bookmarks/{series_id}", patch(update_bookmark_note))
}
```

All routes require `AuthUser` (JWT or API key — middleware already accepts both).

### `GET /bookmarks`

Returns the user's bookmarks **with hydrated CR series metadata** so the frontend renders in one roundtrip.

```rust
#[derive(Serialize, ToSchema)]
pub struct BookmarkItem {
    pub series_id: String,
    pub note: String,
    pub created_at: String,
    pub updated_at: String,
    /// `None` if the CR fetch failed (deleted/region-locked series). Frontend
    /// shows a fallback card so the user can still un-bookmark it.
    pub series: Option<SeriesPreview>,
}

#[derive(Serialize, ToSchema)]
pub struct SeriesPreview {
    pub id: String,
    pub title: String,
    pub description: String,
    pub images: serde_json::Value,  // pass through CR's images shape
}
```

Implementation:
1. `list_bookmarks(&pool, &user.user_id)` → `Vec<BookmarkRow>`.
2. Spawn parallel CR `get_series` fetches via `futures::future::join_all` (existing routes already use this pattern).
3. Map each row to a `BookmarkItem`; on individual fetch error, log + return `series: None` instead of failing the whole request.
4. Order by `created_at DESC` (already indexed).

### `POST /bookmarks`

Request:
```json
{ "series_id": "G6NQ5DWZ6", "note": "" }
```

- Validate: `series_id` non-empty; `note` ≤ 1000 chars.
- `upsert_bookmark(...)` — idempotent. POSTing an already-bookmarked series with a different `note` updates the note (matches the "click star while edit popover open" UX).
- Return **200** with the saved row (no series hydration needed here — the frontend already has the series metadata from wherever the user clicked star).

### `DELETE /bookmarks/{series_id}`

- `Ok(true)` → **204**.
- `Ok(false)` → map to `ApiError::NotFound`.

### `PATCH /bookmarks/{series_id}`

Request:
```json
{ "note": "still need to finish s2" }
```

- Validate: `note` ≤ 1000 chars (empty string allowed — clears the note).
- `update_note(...)`; `Ok(false)` → 404.
- Return **200** with the updated row.

### Wire into router

In `src/routes/mod.rs`:
```rust
pub mod bookmarks;
…
.merge(bookmarks::router())
```

---

## 4. utoipa docs — `src/docs.rs`

- Add the three handler functions to `paths(...)`.
- Add `BookmarkItem`, `SeriesPreview`, `CreateBookmarkRequest`, `UpdateBookmarkRequest` to `components(schemas(...))`.
- Add tag: `(name = "Bookmarks", description = "Per-user saved series")`.
- Annotate handlers with `security(("bearer_auth" = []), ("api_key" = []))`.

---

## 5. Frontend API client — `crunchy-web/lib/api/calls/bookmarks.ts`

```ts
import { get, post, patch, del } from '@/lib/api/client';
import type { CRImages } from '@/types/crunchyroll';

export type BookmarkItem = {
  series_id: string;
  note: string;
  created_at: string;
  updated_at: string;
  series: {
    id: string;
    title: string;
    description: string;
    images: CRImages;
  } | null;
};

export type Bookmark = {
  user_id: string;
  series_id: string;
  note: string;
  created_at: string;
  updated_at: string;
};

export const listBookmarks = (token: string) =>
  get<BookmarkItem[]>(token, '/bookmarks');

export const createBookmark = (token: string, series_id: string, note = '') =>
  post<Bookmark>(token, '/bookmarks', { series_id, note });

export const deleteBookmark = (token: string, series_id: string) =>
  del<void>(token, `/bookmarks/${encodeURIComponent(series_id)}`);

export const updateBookmarkNote = (
  token: string, series_id: string, note: string
) =>
  patch<Bookmark>(token, `/bookmarks/${encodeURIComponent(series_id)}`, { note });
```

---

## 6. Frontend hooks — `crunchy-web/hooks/use-bookmarks.ts`

Three hooks:

```ts
export function useBookmarks();          // useQuery wrapping listBookmarks
export function useToggleBookmark();     // execute(seriesId, isBookmarked)
export function useUpdateBookmarkNote(); // execute(seriesId, note)
```

`useToggleBookmark`'s `execute(seriesId, currentlyBookmarked)`:
- if `currentlyBookmarked` → `deleteBookmark`, else → `createBookmark`
- success toast (`'Bookmarked'` / `'Removed'`)
- error toast on failure (mirror the pattern in `use-config.ts`)

The hook returns `{ execute, isLoading }`. Caller passes `refetch` from `useBookmarks` (or we share a small in-memory cache via React context — see §10).

---

## 7. Bookmark button — `crunchy-web/components/bookmarks/bookmark-button.tsx`

```tsx
type Props = {
  seriesId: string;
  size?: 'sm' | 'icon-sm' | 'default';
  variant?: 'default' | 'ghost' | 'outline';
};
```

- Reads `useBookmarks()` to know the current state (presence of `seriesId` in the list).
- Renders a star icon; filled when bookmarked, outline when not.
- onClick → `useToggleBookmark().execute(seriesId, isBookmarked)` → on success `refetch()`.
- Optimistic UI: flip the icon immediately; revert on error.

---

## 8. `/bookmarks` page — `crunchy-web/app/(protected)/bookmarks/page.tsx`

Layout: PagePanel + PageHeader (title "Bookmarks") + grid of cards.

**Card** (new component `BookmarkCard` in `components/bookmarks/`):
- Poster (CRImage from `item.series.images`, or grey placeholder if `item.series === null`)
- Title (links to `/series/{series_id}`)
- Inline note: a single-line `<input>` styled minimally; blur or Enter saves via `useUpdateBookmarkNote`
- Star button (top-right corner of the card; reuses the same component to un-bookmark)
- "Series unavailable" copy + "Remove bookmark" CTA when `item.series === null`

**Empty state**: bookmark icon + "No bookmarks yet" + link to `/search`.

Loading skeleton: 6 placeholder cards.

---

## 9. Integration points

### Series page — `app/(protected)/series/[id]/page.tsx`

Add `<BookmarkButton seriesId={id} />` in the page header next to the title (or wherever the series action area is — match the existing layout).

### Search-result cards — `components/search/search-result-card.tsx`

Add a small star button overlaid on the poster (top-right corner, semi-transparent background) **only when `item.type === 'series'`**. Other types (episodes, etc.) are not bookmarkable.

The star button is placed inside the `<Link>` wrapper but stops propagation on click so toggling a bookmark doesn't navigate to the series page.

### Sidebar — `components/layout/sidebar-content.tsx`

Add to `mainNavItems`:
```ts
{ label: 'Bookmarks', href: '/bookmarks', icon: <BookmarkIcon /> },
```

Place after `Search`, before `Downloads`.

---

## 10. Bookmark cache shape

To avoid each `BookmarkButton` triggering its own `/bookmarks` fetch, share a single fetch across the page:

- `useBookmarks` already memoizes via `useQuery`'s ref-counted state. As long as every consumer mounts under the same React tree, multiple `useBookmarks()` calls will dedupe to one network request **per component tree**. That's fine for the series page (one button) and the bookmarks page (no button — just the list).
- For the search results page where many cards each render their own button, factor out a lightweight context: `BookmarkSetContext` provides a `Set<string>` of bookmarked series IDs and a `refetch()`. Provide it once at the search page level; cards consume via `useBookmarkSet()`.
- Skip the context for series-page and bookmarks-page (single consumer); only the search page needs it.

---

## Validation

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo check -p crunchy-api --manifest-path crunchy-cli/Cargo.toml
cargo test -p crunchy-api --manifest-path crunchy-cli/Cargo.toml
cd crunchy-web && npm run build
```

Manual smoke test:
1. Bookmark a series from the series page → star fills, toast appears.
2. `/bookmarks` shows the series with poster + title.
3. Edit the note inline; reload → note persists.
4. Bookmark from search results → star fills on the result card; appears on `/bookmarks`.
5. Un-bookmark from `/bookmarks` → card disappears; revisiting series page shows empty star.
6. Bookmark a series, then forcibly delete its row from CR (simulate region-lock by using a junk `series_id` directly via SQL) → the card on `/bookmarks` shows "Series unavailable" with a working remove button.
7. `curl -H "X-Api-Key: crl_..." http://localhost:8080/bookmarks` returns the list (API-key path works).

---

## What this plan does NOT do

- No `+N new` badge / new-episode tracking. Deferred — would need `last_seen_episode_count` snapshot + re-fetch on view.
- No status workflow (`watching` / `completed` / `dropped`). Out of scope.
- No batched / cached CR series fetches. The `GET /bookmarks` handler does N parallel CR calls. Acceptable for dozens of bookmarks; we'll add caching only if it becomes slow.
- No notifications, no episode-level progress tracking.

These can layer on later without a schema change beyond adding columns or a sidecar table.
