import { get, patch } from '@/lib/api/client';

export type AppConfig = {
  // Download preferences
  video_quality: 'best' | '1080p' | '720p' | '480p' | '360p';
  simultaneous_downloads: number;
  parallel_segments: number;
  max_speed_kbps: number | null;
  retry_count: number;

  // Language preferences
  audio_languages: string[];
  subtitle_languages: string[];
  closed_captions: boolean;

  // Muxing options
  output_format: 'mkv' | 'mp4';
  embed_subtitles: boolean;
  default_audio_track: string;
  default_subtitle_track: string;
  prefer_signs_songs: boolean;
  filename_template: string;

  // Advanced
  output_dir: string;
  cache_retention_days: number;
  concurrent_key_acquisitions: number;
  proxy_enabled: boolean;
  proxy_url: string;
  widevine_client: string;
  widevine_private_key: string;

  // Where finished downloads land. `kind=local` falls back to `output_dir`
  // semantics; `kind=s3` uploads via `OutputSink::S3Sink`.
  storage: StorageConfig;
};

export type StorageConfig = {
  kind: 'local' | 's3';
  output_dir: string;
  bucket: string;
  region: string;
  endpoint: string;
  prefix: string;
  access_key_id: string;
  /**
   * The server returns `********` whenever a real secret is stored. PATCH
   * passes that placeholder through unchanged to keep the existing value.
   */
  secret_access_key: string;
  force_path_style: boolean;
};

export const getConfig = (token: string) =>
  get<AppConfig>(token, '/config');

export const updateConfig = (token: string, data: Partial<AppConfig>) =>
  patch<AppConfig>(token, '/config', data);
