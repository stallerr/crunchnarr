'use client';

import { useState, useCallback } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { useQuery } from '@/hooks/use-query';
import {
  listApiKeys,
  createApiKey,
  revokeApiKey,
  type ApiKeyItem,
} from '@/lib/api/calls/api-keys';
import { unwrap } from '@/lib/api/client';
import { toastManager } from '@/components/ui/toast';

export function useApiKeys() {
  return useQuery<ApiKeyItem[]>((token) => listApiKeys(token), []);
}

export function useCreateApiKey() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (name: string) => {
      if (!isAuthenticated) return { data: null, error: 'Not authenticated' };

      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { data: null, error: 'Not authenticated' };

        const { data, error } = await unwrap(createApiKey(token, name));
        return { data, error };
      } catch {
        return { data: null, error: 'An unexpected error occurred' };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { execute, isLoading };
}

export function useRevokeApiKey() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (id: string) => {
      if (!isAuthenticated) return { data: null, error: 'Not authenticated' };

      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { data: null, error: 'Not authenticated' };

        const { data, error } = await unwrap(revokeApiKey(token, id));
        if (!error) {
          toastManager.add({
            title: 'API key revoked',
            type: 'success',
            timeout: 3000,
          });
        }
        return { data, error };
      } catch {
        return { data: null, error: 'An unexpected error occurred' };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { execute, isLoading };
}
