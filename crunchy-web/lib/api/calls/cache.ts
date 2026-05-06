import { get, del } from '@/lib/api/client';

export type CacheEntry = {
  episode_id: string;
  phase: string;
  size_bytes: number;
  created_at: string;
};

export type CacheSummary = {
  total_size_bytes: number;
  entry_count: number;
  retention_days: number;
  entries: CacheEntry[];
};

export const listCache = (token: string) =>
  get<CacheSummary>(token, '/cache');

export const cleanCache = (token: string) =>
  del<{ deleted: number }>(token, '/cache');
