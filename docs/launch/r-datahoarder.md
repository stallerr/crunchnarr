# r/datahoarder post draft

**Title:** `[Tool] Crunchnarr — self-hosted Crunchyroll watchlist that auto-redownloads when new dub tracks drop`

**Flair:** `Software` (sub-specific — change if not a flair)

---

## Body

Wrote a self-hosted server that watches Crunchyroll series and downloads new episodes automatically. Open-sourced it: https://github.com/stallerr/crunchnarr

The bit r/datahoarder might care about: it does **dub-upgrade detection**. If you originally archived an episode sub-only and the JP/EN dub later goes live on CR, the worker spots the missing audio track on its next poll and re-downloads. 24 h cooldown so it doesn't pummel CR while waiting for a dub that hasn't released yet. If the re-download brings new tracks → drop the old. If same tracks → drop the redundant new row.

Other bits worth flagging for archivists:

- **S3-compatible publish target.** Native `OutputSink` for MinIO / Backblaze B2 / R2, alongside local FS. Files land at the configured filename template directly on the destination — segment downloads use a local temp dir, then copy/rename to the final location. NAS-friendly: when source/dest cross filesystems (SMB/NFS), it falls back from `rename(2)` to a byte-stream copy without metadata preservation, so SMB shares that reject `copyfile`-style ACL/xattr ops still work.

- **Filename template.** Plex/Jellyfin-style presets out of the box: `{series}/Season {season:02}/{series} - S{season:02}E{episode:02} - {title}` and a year-prefixed variant for shows that need it. Drag-and-drop builder if you want to roll your own.

- **Manual mark-as-downloaded.** Per-episode and per-season. For all the stuff you already have from BluRay rips / yt-dlp / wherever — flag it and the watchlist worker leaves it alone, no duplicate downloads.

- **Resume cache.** Per-episode `cache.json` + verified segments live at `<temp_dir>/<episode_id>/`. Mid-pipeline failures (segment fetch, decrypt, mux, publish) leave the cache intact; retry resumes from where it died instead of re-fetching every segment. Cache is wiped only after the publish step actually succeeds.

- **API keys** for ingestion automation. Same shape as Sonarr — `X-Api-Key: crl_…`, set up in Settings, hits every protected endpoint.

- **OpenAPI spec** at `/docs`. Swagger UI included. Multi-user: each account has its own CR link + settings + watchlist.

### Caveats

- You supply your own Widevine L3 CDM. The project does not ship one and there's no DRM-bypass path in the code beyond what your own CDM negotiates. Crunchnarr is purely an automation layer over the official streaming pipeline.
- Single-arch image at launch (linux/amd64). Multi-arch is on the list once people start asking.
- TOS-wise: this is a personal-archive tool. CR's terms may prohibit ripping. Use only with content you can legitimately access on your own account.

Stack: Rust API server, Next.js web UI, SQLite. MIT licensed. Repo: https://github.com/stallerr/crunchnarr

---

## When posting

- This sub responds well to dryer, factual writeups — keep marketing copy minimal.
- Lead with the dub-upgrade and resume-cache features (the differentiated archival bits).
- If asked about hash verification: yes — segments are SHA-256 verified in the cache, and segment failures retry; full-file checksums on the muxed output are NOT computed (FFmpeg just writes; you can md5 it externally if you need a manifest).
