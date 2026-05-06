'use client';

import { useState, useCallback } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { useQuery } from '@/hooks/use-query';
import {
  listBookmarks,
  createBookmark,
  deleteBookmark,
  updateBookmarkNote,
  type BookmarkItem,
} from '@/lib/api/calls/bookmarks';
import { unwrap } from '@/lib/api/client';
import { toastManager } from '@/components/ui/toast';

export function useBookmarks() {
  return useQuery<BookmarkItem[]>((token) => listBookmarks(token), []);
}

export function useToggleBookmark() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (seriesId: string, currentlyBookmarked: boolean) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };

      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };

        if (currentlyBookmarked) {
          const { error } = await unwrap(deleteBookmark(token, seriesId));
          if (error) {
            toastManager.add({
              title: 'Failed to remove bookmark',
              description: error,
              type: 'error',
              timeout: 4000,
            });
            return { error };
          }
          toastManager.add({
            title: 'Bookmark removed',
            type: 'success',
            timeout: 2000,
          });
          return { error: null };
        } else {
          const { error } = await unwrap(createBookmark(token, seriesId));
          if (error) {
            toastManager.add({
              title: 'Failed to add bookmark',
              description: error,
              type: 'error',
              timeout: 4000,
            });
            return { error };
          }
          toastManager.add({
            title: 'Bookmarked',
            type: 'success',
            timeout: 2000,
          });
          return { error: null };
        }
      } catch {
        const error = 'An unexpected error occurred';
        toastManager.add({
          title: error,
          type: 'error',
          timeout: 4000,
        });
        return { error };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { execute, isLoading };
}

export function useUpdateBookmarkNote() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (seriesId: string, note: string) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };

      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };

        const { error } = await unwrap(updateBookmarkNote(token, seriesId, note));
        if (error) {
          toastManager.add({
            title: 'Failed to save note',
            description: error,
            type: 'error',
            timeout: 4000,
          });
          return { error };
        }
        return { error: null };
      } catch {
        const error = 'An unexpected error occurred';
        toastManager.add({
          title: error,
          type: 'error',
          timeout: 4000,
        });
        return { error };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { execute, isLoading };
}
