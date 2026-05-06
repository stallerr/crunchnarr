import { get, post, patch, del } from '@/lib/api/client';

export type TrackingMode = 'new_only' | 'all';

export type TrackedSeriesItem = {
  id: string;
  series_id: string;
  series_title: string;
  series_thumbnail: string | null;
  download_mode: TrackingMode;
  enabled: boolean;
  added_at: string;
  last_checked_at: string | null;
};

export type CheckSummary = {
  new_downloads: number;
  upgrades: number;
  checked_episodes: number;
};

export const listTracked = (token: string) =>
  get<TrackedSeriesItem[]>(token, '/tracking');

export const addTracked = (
  token: string,
  series_id: string,
  download_mode: TrackingMode
) => post<TrackedSeriesItem>(token, '/tracking', { series_id, download_mode });

export const updateTracked = (
  token: string,
  id: string,
  body: { download_mode?: TrackingMode; enabled?: boolean }
) => patch<TrackedSeriesItem>(token, `/tracking/${encodeURIComponent(id)}`, body);

export const deleteTracked = (token: string, id: string) =>
  del<void>(token, `/tracking/${encodeURIComponent(id)}`);

export const checkTracked = (token: string, id: string) =>
  post<CheckSummary>(token, `/tracking/${encodeURIComponent(id)}/check`);
