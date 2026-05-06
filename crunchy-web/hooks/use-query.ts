'use client';

import { useState, useEffect, useCallback, useRef } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import type { Result } from '@/types/api';

type UseQueryOptions = {
  enabled?: boolean;
};

type UseQueryResult<T> = {
  data: T | null;
  error: string | null;
  isLoading: boolean;
  refetch: () => Promise<void>;
};

export function useQuery<T>(
  fetcher: (token: string) => Promise<Result<T>>,
  deps: unknown[] = [],
  options: UseQueryOptions = {}
): UseQueryResult<T> {
  const { getToken, isAuthenticated } = useAuthToken();
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const mountedRef = useRef(true);

  const { enabled = true } = options;

  const fetchData = useCallback(async () => {
    if (!isAuthenticated || !enabled) {
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const token = await getToken();
      if (!token) {
        setError('Not authenticated');
        setIsLoading(false);
        return;
      }

      const result = await fetcher(token);

      if (!mountedRef.current) return;

      if (result.success) {
        setData(result.data);
        setError(null);
      } else {
        setError(
          result.data && typeof result.data === 'object' && 'error' in result.data
            ? (result.data as { error: string }).error
            : 'Request failed'
        );
      }
    } catch {
      if (mountedRef.current) {
        setError('An unexpected error occurred');
      }
    } finally {
      if (mountedRef.current) {
        setIsLoading(false);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isAuthenticated, enabled, getToken, ...deps]);

  useEffect(() => {
    mountedRef.current = true;
    fetchData();
    return () => {
      mountedRef.current = false;
    };
  }, [fetchData]);

  return { data, error, isLoading, refetch: fetchData };
}
