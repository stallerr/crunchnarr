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

export type DownloadResponse = {
  id: string;
  status: DownloadStatus;
  episode_id: string;
  episode_title: string;
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
