# /r/animepiracy post draft

**Title:** `Crunchnarr — Sonarr-for-Crunchyroll: watchlist + auto-download + auto dub re-grabs`

**Flair:** check sub-specific options (`Tool` / `Release` / similar)

---

## Body

If you're running an *Arr stack on a NAS and have always had a hole where Crunchyroll fits, this might help: https://github.com/stallerr/crunchnarr

It's a self-hosted server (Rust API + Next.js web UI) that:

- **Tracks series.** Add via the UI or `POST /tracking`, pick `new_only` (only future episodes) or `all` (backfill). Worker polls every N minutes (configurable, default 60).
- **Auto-downloads new episodes.** Honors your saved language settings (audio languages + subs), filename template, output dir.
- **Re-downloads when dubs drop late.** Originally grabbed sub-only? Worker checks completed downloads on each poll, sees the missing audio track if CR has added it since, supersedes the old row, queues a re-download. 24 h cooldown so it doesn't slam CR while waiting.
- **Manual mark-as-downloaded.** For episodes you grabbed elsewhere — flag them per-episode or per-season and the worker leaves them alone.

### What it isn't

- Not a CDM extractor. You provide your own Widevine L3 client_id + private_key.
- Not a "click button → magic" desktop app like Crunchy-Downloader. Headless server, runs in Docker, web UI you reach from any device on the LAN.
- Not multi-source — Crunchyroll only.

### Setup

Drop your `client_id.bin` + `private_key.pem` into a `./widevine` dir, then:

```bash
docker run -d \
  --network host \
  -e PORT=8080 -e HOST=0.0.0.0 \
  -e DATABASE_URL=sqlite:/data/crunchy-api.db?mode=rwc \
  -e JWT_SECRET=$(openssl rand -base64 32) \
  -e STORAGE_SECRET_KEY=$(openssl rand -hex 32) \
  -e API_URL=http://localhost:8080 \
  -v ./data:/data \
  -v ./downloads:/downloads \
  -v ./widevine:/widevine:ro \
  ghcr.io/stallerr/crunchnarr:latest
```

(Image is multi-arch: `linux/amd64` + `linux/arm64`.)

UI at `http://localhost:3000`. Register an account, link your CR account in **Account → Crunchyroll**, then add a series. The mounted CDM at `/widevine` is read by the API on demand — no in-UI upload needed if it's mounted.

### Disclaimer

You provide the Widevine CDM. The project does not bundle one and there's no DRM-bypass path in the code beyond what your own CDM negotiates. Crunchnarr is purely an automation layer over the official streaming pipeline. Use only for content you can legitimately access on your own Crunchyroll account. CR's TOS may prohibit ripping; this is a personal-archive tool with no warranty.

### Stack

Rust + Tokio + Axum on the API, Next.js 16 + Tailwind on the UI, SQLite for storage. MIT licensed.

Repo + docs: https://github.com/stallerr/crunchnarr

Open to issues / PRs. Multi-arch image, a few more filename presets, and Synology / Unraid setup notes are the next priorities.

---

## When posting

- This community is more permissive about ripping context than r/selfhosted; you don't need to over-explain the legal bit, but still include the "you supply your own CDM" line so it's clear the project itself isn't shipping the bypass.
- Mention what it competes with (Crunchy-Downloader, devine, etc.) and lean into the *Arr ergonomic as the differentiator.
- If asked about non-CR sources (Funimation, HIDIVE, etc.): "Crunchyroll only for now. Adding more sources is a structural change to the CR client; PRs welcome."
