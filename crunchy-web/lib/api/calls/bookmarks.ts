import { get, post, patch, del } from '@/lib/api/client';
import type { CRImages } from '@/types/crunchyroll';

export type BookmarkSeriesPreview = {
  id: string;
  title: string;
  description: string;
  images: CRImages;
};

export type BookmarkItem = {
  series_id: string;
  note: string;
  created_at: string;
  updated_at: string;
  /** `null` when the CR fetch failed (deleted/region-locked series). */
  series: BookmarkSeriesPreview | null;
};

export type Bookmark = {
  user_id: string;
  series_id: string;
  note: string;
  created_at: string;
  updated_at: string;
};

export const listBookmarks = (token: string) =>
  get<BookmarkItem[]>(token, '/bookmarks');

export const createBookmark = (token: string, series_id: string, note = '') =>
  post<Bookmark>(token, '/bookmarks', { series_id, note });

export const deleteBookmark = (token: string, series_id: string) =>
  del<void>(token, `/bookmarks/${encodeURIComponent(series_id)}`);

export const updateBookmarkNote = (
  token: string,
  series_id: string,
  note: string
) =>
  patch<Bookmark>(
    token,
    `/bookmarks/${encodeURIComponent(series_id)}`,
    { note }
  );
