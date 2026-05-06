'use client';

import { useMemo, useState, useCallback } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { useQuery } from '@/hooks/use-query';
import {
  listTracked,
  addTracked,
  updateTracked,
  deleteTracked,
  checkTracked,
  type TrackedSeriesItem,
  type TrackingMode,
  type CheckSummary,
} from '@/lib/api/calls/tracking';
import { unwrap } from '@/lib/api/client';
import { toastManager } from '@/components/ui/toast';

export function useTrackedSeries() {
  const result = useQuery<TrackedSeriesItem[]>((token) => listTracked(token), []);
  const bySeriesId = useMemo(() => {
    const m = new Map<string, TrackedSeriesItem>();
    for (const row of result.data ?? []) m.set(row.series_id, row);
    return m;
  }, [result.data]);
  return { ...result, bySeriesId };
}

export function useAddTracked() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (seriesId: string, mode: TrackingMode) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { data, error } = await unwrap(addTracked(token, seriesId, mode));
        if (error) {
          toastManager.add({
            title: 'Failed to add to watchlist',
            description: error,
            type: 'error',
            timeout: 4000,
          });
          return { error };
        }
        toastManager.add({
          title: 'Added to watchlist',
          type: 'success',
          timeout: 2500,
        });
        return { data, error: null };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { execute, isLoading };
}

export function useUpdateTracked() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (id: string, body: { download_mode?: TrackingMode; enabled?: boolean }) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { data, error } = await unwrap(updateTracked(token, id, body));
        if (error) {
          toastManager.add({
            title: 'Failed to update watchlist entry',
            description: error,
            type: 'error',
            timeout: 4000,
          });
          return { error };
        }
        return { data, error: null };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { execute, isLoading };
}

export function useDeleteTracked() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (id: string) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { error } = await unwrap(deleteTracked(token, id));
        if (error) {
          toastManager.add({
            title: 'Failed to remove from watchlist',
            description: error,
            type: 'error',
            timeout: 4000,
          });
          return { error };
        }
        toastManager.add({
          title: 'Removed from watchlist',
          type: 'success',
          timeout: 2500,
        });
        return { error: null };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { execute, isLoading };
}

export function useCheckTracked() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (id: string): Promise<{ summary?: CheckSummary; error: string | null }> => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { data, error } = await unwrap(checkTracked(token, id));
        if (error || !data) {
          toastManager.add({
            title: 'Check failed',
            description: error ?? 'Unknown error',
            type: 'error',
            timeout: 4000,
          });
          return { error: error ?? 'Unknown error' };
        }
        const total = data.new_downloads + data.upgrades;
        const description =
          total > 0
            ? `${data.new_downloads} new, ${data.upgrades} upgrade · ${data.checked_episodes} episodes scanned`
            : `Up to date · ${data.checked_episodes} episodes scanned`;
        toastManager.add({
          title: total > 0 ? `Started ${total} download${total === 1 ? '' : 's'}` : 'Check complete',
          description,
          type: total > 0 ? 'success' : 'info',
          timeout: 4000,
        });
        return { summary: data, error: null };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { execute, isLoading };
}
