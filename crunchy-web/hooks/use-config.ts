'use client';

import { useState, useCallback } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { useQuery } from '@/hooks/use-query';
import { getConfig, updateConfig } from '@/lib/api/calls/config';
import { unwrap } from '@/lib/api/client';
import { toastManager } from '@/components/ui/toast';
import type { AppConfig } from '@/lib/api/calls/config';

export function useConfig() {
  return useQuery<AppConfig>((token) => getConfig(token), []);
}

export function useUpdateConfig() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (data: Partial<AppConfig>) => {
      if (!isAuthenticated) return { data: null, error: 'Not authenticated' };

      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { data: null, error: 'Not authenticated' };

        const { data: result, error } = await unwrap(updateConfig(token, data));
        if (result && !error) {
          toastManager.add({
            title: 'Settings saved',
            type: 'success',
            timeout: 3000,
          });
        } else {
          toastManager.add({
            title: 'Failed to save settings',
            description: error ?? 'Unknown error',
            type: 'error',
            timeout: 5000,
          });
        }
        return { data: result, error };
      } catch (err) {
        toastManager.add({
          title: 'Failed to save settings',
          description: err instanceof Error ? err.message : 'Unexpected error',
          type: 'error',
          timeout: 5000,
        });
        return { data: null, error: 'An unexpected error occurred' };
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated]
  );

  return { execute, isLoading };
}
