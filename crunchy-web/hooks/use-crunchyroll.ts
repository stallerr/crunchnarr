'use client';

import { useState, useCallback } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { useQuery } from '@/hooks/use-query';
import {
  getCrunchyrollProfile,
  loginCrunchyroll,
  logoutCrunchyroll,
} from '@/lib/api/calls/crunchyroll';
import type { CRProfile } from '@/types/crunchyroll';

export function useCrunchyrollStatus() {
  const { data, error, isLoading, refetch } = useQuery<CRProfile>(
    (token) => getCrunchyrollProfile(token),
    []
  );

  return {
    isLinked: data !== null && !error,
    isLoading,
    profile: data,
    error,
    refetch,
  };
}

export function useCrunchyrollLogin() {
  const { getToken } = useAuthToken();
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const login = useCallback(
    async (username: string, password: string) => {
      setError(null);
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) {
          setError('Not authenticated');
          return false;
        }
        const result = await loginCrunchyroll(token, { username, password });
        if (result.success) return true;
        setError(
          result.data && typeof result.data === 'object' && 'error' in result.data
            ? (result.data as { error: string }).error
            : 'Failed to link Crunchyroll account'
        );
        return false;
      } catch {
        setError('An unexpected error occurred');
        return false;
      } finally {
        setIsLoading(false);
      }
    },
    [getToken]
  );

  const loginWithToken = useCallback(
    async (refreshToken: string) => {
      setError(null);
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) {
          setError('Not authenticated');
          return false;
        }
        const result = await loginCrunchyroll(token, { refresh_token: refreshToken });
        if (result.success) return true;
        setError(
          result.data && typeof result.data === 'object' && 'error' in result.data
            ? (result.data as { error: string }).error
            : 'Failed to link with refresh token'
        );
        return false;
      } catch {
        setError('An unexpected error occurred');
        return false;
      } finally {
        setIsLoading(false);
      }
    },
    [getToken]
  );

  const unlink = useCallback(async () => {
    setError(null);
    setIsLoading(true);
    try {
      const token = await getToken();
      if (!token) return false;
      const result = await logoutCrunchyroll(token);
      return result.success;
    } catch {
      setError('Failed to unlink account');
      return false;
    } finally {
      setIsLoading(false);
    }
  }, [getToken]);

  return { login, loginWithToken, unlink, error, setError, isLoading };
}
