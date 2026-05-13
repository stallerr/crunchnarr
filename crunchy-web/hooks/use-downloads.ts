'use client';

import { useMemo, useState, useCallback, useEffect, useRef } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { useQuery } from '@/hooks/use-query';
import { useWebSocketSubscription } from '@/hooks/use-websocket';
import {
  getDownloads,
  getDownloadCounts,
  getDownloadedEpisodeIds,
  startDownload,
  cancelDownload,
  cancelActiveDownloads,
  pauseDownload,
  resumeDownload,
  markManual,
  markManualBulk,
  unmarkManual,
  type DownloadedEpisodeIds,
  type MarkManualItem,
} from '@/lib/api/calls/downloads';
import { unwrap } from '@/lib/api/client';
import { toastManager } from '@/components/ui/toast';
import type { DownloadOptions } from '@/types/download-options';
import type {
  DownloadRow,
  DownloadCounts,
  DownloadResponse,
  SkipReason,
} from '@/types/downloads';

type TabStatus = 'active' | 'completed' | 'failed' | 'cancelled' | undefined;

export function useInfiniteDownloads(status?: TabStatus) {
  const { getToken, isAuthenticated } = useAuthToken();
  const [items, setItems] = useState<DownloadRow[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingMore, setIsLoadingMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mountedRef = useRef(true);
  // Track current status to detect changes
  const statusRef = useRef(status);

  const fetchPage = useCallback(
    async (cursor?: string) => {
      if (!isAuthenticated) return;

      const token = await getToken();
      if (!token) return;

      const result = await getDownloads(token, {
        status,
        cursor: cursor ?? undefined,
        limit: 50,
      });

      if (!mountedRef.current) return;

      if (result.success) {
        const page = result.data;
        if (cursor) {
          // Appending next page
          setItems((prev) => [...prev, ...page.items]);
        } else {
          // Fresh fetch (first page)
          setItems(page.items);
        }
        setNextCursor(page.next_cursor);
        setHasMore(page.has_more);
        setError(null);
      } else {
        const msg =
          result.data && typeof result.data === 'object' && 'error' in result.data
            ? (result.data as { error: string }).error
            : 'Request failed';
        setError(msg);
      }
    },
    [isAuthenticated, getToken, status]
  );

  // Initial load + reset when status changes
  useEffect(() => {
    mountedRef.current = true;
    statusRef.current = status;
    setIsLoading(true);
    setItems([]);
    setNextCursor(null);
    setHasMore(false);

    fetchPage().finally(() => {
      if (mountedRef.current) setIsLoading(false);
    });

    return () => {
      mountedRef.current = false;
    };
  }, [fetchPage]);

  const loadMore = useCallback(async () => {
    if (!hasMore || isLoadingMore || !nextCursor) return;
    setIsLoadingMore(true);
    await fetchPage(nextCursor);
    if (mountedRef.current) setIsLoadingMore(false);
  }, [hasMore, isLoadingMore, nextCursor, fetchPage]);

  // On WS events, reset to first page to pick up changes
  const reset = useCallback(() => {
    setItems([]);
    setNextCursor(null);
    setHasMore(false);
    setIsLoading(true);
    fetchPage().finally(() => {
      if (mountedRef.current) setIsLoading(false);
    });
  }, [fetchPage]);

  useWebSocketSubscription('download_complete', reset);
  useWebSocketSubscription('download_failed', reset);

  /**
   * Mutate a single row in place — used by cancel/pause/resume so the row
   * doesn't have to refetch the whole list and lose scroll position.
   * When `update` returns null the row is removed; otherwise it's replaced.
   */
  const mutateRow = useCallback(
    (id: string, update: (row: DownloadRow) => DownloadRow | null) => {
      setItems((prev) =>
        prev.flatMap((row) => {
          if (row.id !== id) return [row];
          const next = update(row);
          return next ? [next] : [];
        })
      );
    },
    []
  );

  return {
    items,
    isLoading,
    isLoadingMore,
    error,
    hasMore,
    loadMore,
    refetch: reset,
    mutateRow,
  };
}

export function useDownloadCounts() {
  const result = useQuery<DownloadCounts>(
    (token) => getDownloadCounts(token),
    []
  );

  useWebSocketSubscription('download_complete', () => result.refetch());
  useWebSocketSubscription('download_failed', () => result.refetch());

  return result;
}

/**
 * Returns two Sets — episodes with a real completed download and episodes the
 * user manually marked. Auto-refetches on `download_complete` websocket events.
 *
 * The combined `ids` set is exposed for backward compatibility with consumers
 * that just want to know whether an episode has *any* downloaded badge to show.
 */
export function useDownloadedEpisodes() {
  const result = useQuery<DownloadedEpisodeIds>(
    (token) => getDownloadedEpisodeIds(token),
    []
  );

  useWebSocketSubscription('download_complete', () => result.refetch());

  const completedIds = useMemo(
    () => new Set(result.data?.completed ?? []),
    [result.data]
  );
  const manualIds = useMemo(
    () => new Set(result.data?.manual ?? []),
    [result.data]
  );
  const ids = useMemo(() => {
    const merged = new Set<string>(completedIds);
    for (const id of manualIds) merged.add(id);
    return merged;
  }, [completedIds, manualIds]);

  return {
    ids,
    completedIds,
    manualIds,
    isLoading: result.isLoading,
    refetch: result.refetch,
  };
}

export function useMarkManual() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const mark = useCallback(
    async (item: MarkManualItem) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { data, error } = await unwrap(markManual(token, item));
        if (error) {
          toastManager.add({
            title: 'Failed to mark as downloaded',
            description: error,
            type: 'error',
            timeout: 4000,
          });
          return { error };
        }
        if (data?.skipped) {
          toastManager.add({
            title: 'Already downloaded',
            description: 'This episode already has a real download.',
            type: 'info',
            timeout: 3000,
          });
        } else {
          toastManager.add({
            title: 'Marked as downloaded',
            type: 'success',
            timeout: 2500,
          });
        }
        return { data, error: null };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  const markBulk = useCallback(
    async (items: MarkManualItem[]) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      if (items.length === 0) return { error: null, data: { marked: 0, skipped: 0 } };
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { data, error } = await unwrap(markManualBulk(token, items));
        if (error) {
          toastManager.add({
            title: 'Failed to mark season',
            description: error,
            type: 'error',
            timeout: 4000,
          });
          return { error };
        }
        const marked = data?.marked ?? 0;
        const skipped = data?.skipped ?? 0;
        toastManager.add({
          title: marked > 0 ? `Marked ${marked} episode${marked === 1 ? '' : 's'}` : 'Nothing to mark',
          description: skipped > 0
            ? `${skipped} already had a real download${skipped === 1 ? '' : 's'}.`
            : undefined,
          type: marked > 0 ? 'success' : 'info',
          timeout: 3000,
        });
        return { data, error: null };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  const unmark = useCallback(
    async (episodeId: string) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { error } = await unwrap(unmarkManual(token, episodeId));
        if (error) {
          toastManager.add({
            title: 'Failed to unmark',
            description: error,
            type: 'error',
            timeout: 4000,
          });
          return { error };
        }
        toastManager.add({
          title: 'Unmarked',
          type: 'success',
          timeout: 2000,
        });
        return { error: null };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { mark, markBulk, unmark, isLoading };
}

export function useStartDownload() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const fire = useCallback(
    async (episodeId: string, options: DownloadOptions, force: boolean) => {
      const token = await getToken();
      if (!token) return { data: null, error: 'Not authenticated' };
      const url = `https://www.crunchyroll.com/watch/${episodeId}`;
      return unwrap(startDownload(token, url, options, force));
    },
    [getToken]
  );

  const execute = useCallback(
    async (episodeId: string, options: DownloadOptions, force = false) => {
      if (!isAuthenticated) return { data: null, error: 'Not authenticated' };

      setIsLoading(true);
      try {
        const { data, error } = await fire(episodeId, options, force);
        if (error || !data) return { data, error };

        const started = data.filter((d) => d.status === 'pending');
        const skipped = data.filter((d) => d.status === 'skipped');

        if (started.length > 0 && skipped.length === 0) {
          toastManager.add({
            title: 'Download started',
            type: 'info',
            timeout: 5000,
          });
        } else if (started.length === 0 && skipped.length === 1) {
          // Single-episode click on something already present. Offer to
          // re-download via force=true.
          toastManager.add({
            title: skipTitle(skipped[0].skip_reason),
            description: skipDescription(skipped[0]),
            type: 'info',
            timeout: 8000,
            actionProps: {
              children: 'Re-download',
              onClick: () => {
                void execute(episodeId, options, true);
              },
            },
          });
        } else {
          toastManager.add({
            title: `Download started`,
            description: `${started.length} new, ${skipped.length} skipped`,
            type: 'info',
            timeout: 6000,
          });
        }

        return { data, error };
      } catch {
        return { data: null, error: 'An unexpected error occurred' };
      } finally {
        setIsLoading(false);
      }
    },
    [isAuthenticated, fire]
  );

  return { execute, isLoading };
}

function skipTitle(reason?: SkipReason): string {
  switch (reason) {
    case 'in_progress':
      return 'Already downloading';
    case 'file_exists':
      return 'Already on disk';
    case 'already_downloaded':
    default:
      return 'Already downloaded';
  }
}

function skipDescription(entry: DownloadResponse): string | undefined {
  if (entry.skip_reason === 'file_exists' && entry.existing_path) {
    return entry.existing_path;
  }
  return undefined;
}

/**
 * Cancel every active/pending/paused download owned by the caller in one
 * shot. Used by the 'Cancel all active' header button on /downloads.
 */
export function useCancelActiveDownloads() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(async () => {
    if (!isAuthenticated) return { data: null, error: 'Not authenticated' };
    setIsLoading(true);
    try {
      const token = await getToken();
      if (!token) return { data: null, error: 'Not authenticated' };
      return await unwrap(cancelActiveDownloads(token));
    } finally {
      setIsLoading(false);
    }
  }, [getToken, isAuthenticated]);

  return { execute, isLoading };
}

export function useDownloadActions() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [loadingId, setLoadingId] = useState<string | null>(null);

  const pause = useCallback(
    async (id: string) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setLoadingId(id);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { error } = await unwrap(pauseDownload(token, id));
        return { error };
      } catch {
        return { error: 'An unexpected error occurred' };
      } finally {
        setLoadingId(null);
      }
    },
    [getToken, isAuthenticated]
  );

  const resume = useCallback(
    async (id: string) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setLoadingId(id);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { error } = await unwrap(resumeDownload(token, id));
        return { error };
      } catch {
        return { error: 'An unexpected error occurred' };
      } finally {
        setLoadingId(null);
      }
    },
    [getToken, isAuthenticated]
  );

  const cancel = useCallback(
    async (id: string) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setLoadingId(id);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { error } = await unwrap(cancelDownload(token, id));
        return { error };
      } catch {
        return { error: 'An unexpected error occurred' };
      } finally {
        setLoadingId(null);
      }
    },
    [getToken, isAuthenticated]
  );

  return { pause, resume, cancel, loadingId };
}
