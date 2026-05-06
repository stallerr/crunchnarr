import { get, post, patch, del } from '@/lib/api/client';
import type { DownloadOptions } from '@/types/download-options';
import type { DownloadRow, DownloadResponse, PaginatedDownloads, DownloadCounts } from '@/types/downloads';

export const getDownloads = (
  token: string,
  params?: { status?: string; cursor?: string; limit?: number }
) => {
  const query = new URLSearchParams();
  if (params?.status) query.set('status', params.status);
  if (params?.cursor) query.set('cursor', params.cursor);
  if (params?.limit) query.set('limit', String(params.limit));
  const qs = query.toString();
  return get<PaginatedDownloads>(token, `/downloads${qs ? `?${qs}` : ''}`);
};

export const getDownloadCounts = (token: string) =>
  get<DownloadCounts>(token, '/downloads/counts');

export type DownloadedEpisodeIds = {
  /** Episode IDs with a real completed download. */
  completed: string[];
  /** Episode IDs the user manually marked as already downloaded. */
  manual: string[];
};

export const getDownloadedEpisodeIds = (token: string) =>
  get<DownloadedEpisodeIds>(token, '/downloads/episode-ids');

export type MarkManualItem = {
  episode_id: string;
  series_title?: string | null;
  episode_title?: string | null;
  season_number?: number | null;
  episode_number?: number | null;
  thumbnail_url?: string | null;
};

export type MarkManualResponse = {
  marked: number;
  skipped: number;
};

export const markManual = (token: string, item: MarkManualItem) =>
  post<MarkManualResponse>(token, '/downloads/manual', item);

export const markManualBulk = (token: string, items: MarkManualItem[]) =>
  post<MarkManualResponse>(token, '/downloads/manual/bulk', { items });

export const unmarkManual = (token: string, episode_id: string) =>
  del<void>(token, `/downloads/manual/${encodeURIComponent(episode_id)}`);

export const getDownload = (token: string, id: string) =>
  get<DownloadRow>(token, `/downloads/${id}`);

export const startDownload = (
  token: string,
  url: string,
  options: DownloadOptions
) => post<DownloadResponse[]>(token, '/downloads', { url, options });

export const cancelDownload = (token: string, id: string) =>
  del<{ status: string }>(token, `/downloads/${id}`);

export const pauseDownload = (token: string, id: string) =>
  patch<{ status: string }>(token, `/downloads/${id}/pause`);

export const resumeDownload = (token: string, id: string) =>
  patch<{ status: string }>(token, `/downloads/${id}/resume`);
