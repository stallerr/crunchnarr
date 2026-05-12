import type { DownloadOptions } from './download-options';

export type DownloadStatus =
  | 'pending'
  | 'active'
  | 'completed'
  | 'failed'
  | 'paused'
  | 'cancelled';

export type DownloadRow = {
  id: string;
  user_id: string;
  episode_id: string;
  series_title: string;
  episode_title: string;
  season_number: number;
  episode_number: number;
  status: DownloadStatus;
  options_json: string;
  progress_json: string;
  output_path: string | null;
  error: string | null;
  thumbnail_url: string | null;
  created_at: string;
  updated_at: string;
};

/** Why a `POST /downloads` entry was skipped. */
export type SkipReason = 'already_downloaded' | 'in_progress' | 'file_exists';

export type DownloadResponse = {
  /** New `downloads.id` when started, or the prior row's UUID when skipped
   *  because a DB row exists. Null when skipped purely because the file
   *  exists at the templated output path (no DB row was created). */
  id: string | null;
  /** `"pending"` for a newly-queued download, `"skipped"` otherwise. */
  status: 'pending' | 'skipped';
  episode_id: string;
  episode_title: string;
  skip_reason?: SkipReason;
  existing_download_id?: string;
  existing_path?: string;
};

export type PaginatedDownloads = {
  items: DownloadRow[];
  next_cursor: string | null;
  has_more: boolean;
};

export type DownloadCounts = {
  all: number;
  active: number;
  completed: number;
  failed: number;
  cancelled: number;
};

export type StartDownloadRequest = {
  url: string;
  options: DownloadOptions;
};
