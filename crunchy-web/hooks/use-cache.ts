'use client';

import { useState, useCallback } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { useQuery } from '@/hooks/use-query';
import { listCache, cleanCache } from '@/lib/api/calls/cache';
import { unwrap } from '@/lib/api/client';
import { toastManager } from '@/components/ui/toast';
import type { CacheSummary } from '@/lib/api/calls/cache';

export function useCache() {
  return useQuery<CacheSummary>((token) => listCache(token), []);
}

export function useCleanCache() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(async () => {
    if (!isAuthenticated) return { data: null, error: 'Not authenticated' };

    setIsLoading(true);
    try {
      const token = await getToken();
      if (!token) return { data: null, error: 'Not authenticated' };

      const { data, error } = await unwrap(cleanCache(token));
      if (data && !error) {
        toastManager.add({
          title: `Cache cleared — ${data.deleted} entries removed`,
          type: 'success',
          timeout: 4000,
        });
      }
      return { data, error };
    } catch {
      return { data: null, error: 'An unexpected error occurred' };
    } finally {
      setIsLoading(false);
    }
  }, [getToken, isAuthenticated]);

  return { execute, isLoading };
}
