import { get, patch } from '@/lib/api/client';

export type AppSettings = {
  /** How often the watchlist worker polls (seconds). */
  tracking_interval_secs: number;
};

export type UpdateAppSettingsRequest = {
  tracking_interval_secs?: number;
};

export const getAppSettings = (token: string) =>
  get<AppSettings>(token, '/app-settings');

export const updateAppSettings = (
  token: string,
  body: UpdateAppSettingsRequest
) => patch<AppSettings>(token, '/app-settings', body);
