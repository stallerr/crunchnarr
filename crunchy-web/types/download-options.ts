/**
 * Per-download overrides on top of the user's saved settings. All fields are
 * optional — sending an empty object means "use saved settings unchanged."
 * Keys must be snake_case to match the backend's allowlist (see
 * `PER_REQUEST_OVERRIDE_KEYS` in `crunchy-cli/crates/api/src/services/download.rs`).
 */
export interface DownloadOptions {
  video_quality?: 'best' | '1080p' | '720p' | '480p' | '360p';
  audio_languages?: string[];
  subtitle_languages?: string[];
  output_format?: 'mkv' | 'mp4';
}

export const DEFAULT_DOWNLOAD_OPTIONS: DownloadOptions = {};
