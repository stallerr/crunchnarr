# r/selfhosted post draft

**Title options (pick one — first reads cleanest for the sub):**

1. `Crunchnarr — a self-hosted Crunchyroll watchlist & downloader (Sonarr-style, with auto dub upgrades)`
2. `[Release] Crunchnarr: a *Arr-style server for tracking Crunchyroll series with auto dub re-downloads`
3. `Built a Sonarr-for-Crunchyroll for my home media setup. Open-sourcing it.`

**Flair:** `Release` if the sub has it, otherwise leave blank.

---

## Body

I run Sonarr/Radarr/Bazarr/Jellyfin on my NAS and there was a hole for Crunchyroll: most Crunchyroll downloaders are single-user Windows GUI apps, not headless servers. I wanted something that fit the same shape as the *Arr stack — runs in a container, web UI, REST API for automation, multi-user.

So I built **Crunchnarr** and just open-sourced it: https://github.com/stallerr/crunchnarr

### What it does

- **Watchlist with auto-download.** Add a series, pick `new_only` (only future episodes) or `all` (backfill + ongoing). The worker polls on a configurable interval (5 min – 24 h) and queues new episodes automatically.
- **Auto dub upgrades.** When you originally grab an episode sub-only and the dub later drops on Crunchyroll, the worker detects the missing audio track on the next poll and re-downloads it. 24 h cooldown so it doesn't hammer CR while waiting for a dub that hasn't released yet. If the re-download brings new tracks → drop the old row. If it has the same tracks → drop the new redundant row, keep the old, and re-cooldown.
- **API keys.** `crl_…` prefix, send as `X-Api-Key`. Same shape as Sonarr's API keys — drop into Home Assistant, shell scripts, anything.
- **Manual mark-as-downloaded.** For episodes you already have on disk from elsewhere (legacy yt-dlp grabs, BluRay rips). Single episodes or a whole season at once. Watchlist worker leaves them alone.
- **S3-compatible publish target.** MinIO / Backblaze / R2 via the `OutputSink` abstraction, or just write to a local mount.
- **Bookmarks.** Save series for later with editable per-series notes. Independent from the watchlist.
- **Multi-user.** Each user has their own Crunchyroll account link, their own settings, their own watchlist. Singleton server settings (like the polling interval) live in their own table.

### Stack

- API: Rust, Axum, sqlx (SQLite), JWT, Argon2, utoipa (OpenAPI/Swagger at `/docs`).
- Web: Next.js 16 / React 19 / Tailwind / Base UI.
- DRM: in-process Widevine L3 (you supply your own CDM — Crunchnarr does not ship one), `mp4decrypt` for segment decryption, FFmpeg for muxing.

### Quick start

```yaml
# docker-compose.yml
services:
  app:
    image: ghcr.io/stallerr/crunchnarr:latest    # multi-arch: amd64 + arm64
    network_mode: host
    environment:
      - PORT=8080
      - HOST=0.0.0.0
      - DATABASE_URL=sqlite:/data/crunchy-api.db?mode=rwc
      - JWT_SECRET=${JWT_SECRET}
      - STORAGE_SECRET_KEY=${STORAGE_SECRET_KEY}
      - DOWNLOADS_DIR=/downloads
      - API_URL=http://localhost:8080
    volumes:
      - api-data:/data
      - ${DOWNLOADS_DIR}:/downloads
      - ${WIDEVINE_DIR}:/widevine:ro

volumes:
  api-data:
```

Generate `JWT_SECRET` (`openssl rand -base64 32`) and `STORAGE_SECRET_KEY` (`openssl rand -hex 32`), point `WIDEVINE_DIR` at your CDM, `docker compose up -d`. UI at `http://localhost:3000`.

### NAS-friendly

If your `DOWNLOADS_DIR` is a mounted SMB/NFS share, the publish step falls back from `rename(2)` to a byte-stream copy when `EXDEV` (cross-filesystem) trips. No metadata preservation — pure bytes.

### Honest disclaimer

You provide the Widevine CDM. The project does not bundle one and there's no path in the code that bypasses DRM you haven't authorized via your own credentials. Use only for content you can legitimately access on your own Crunchyroll account, on devices and networks you own. CR's TOS may prohibit ripping; this is a personal-use tool with no warranty.

### Where it could use help

- Test on Synology / Unraid / TrueNAS native Docker setups (I run macOS for dev).
- More filename-template presets / nicer UI for editing.
- Per-watchlist-entry override for polling cadence (currently global).

Issues/PRs welcome. Repo: https://github.com/stallerr/crunchnarr

---

## When posting

- Lead with the watchlist screenshot. Then series page with the Marked / Tracked badges. Then the "Add Show" search modal.
- Don't reply to "is this legal" questions argumentatively — link the legal section in the README and move on.
- Expect "what about devine / streamlink-binge / cli-equivalent X" comparisons. Acknowledge them, lean on the *Arr-stack ergonomic as the differentiator.
- Image already publishes for `linux/amd64` + `linux/arm64`. Don't promise other arches or scheduled-update features beyond what's shipped.
