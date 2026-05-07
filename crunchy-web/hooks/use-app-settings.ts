'use client';

import { useState, useCallback } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { useQuery } from '@/hooks/use-query';
import {
  getAppSettings,
  updateAppSettings,
  type AppSettings,
  type UpdateAppSettingsRequest,
} from '@/lib/api/calls/app-settings';
import { unwrap } from '@/lib/api/client';
import { toastManager } from '@/components/ui/toast';

export function useAppSettings() {
  return useQuery<AppSettings>((token) => getAppSettings(token), []);
}

export function useUpdateAppSettings() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const execute = useCallback(
    async (body: UpdateAppSettingsRequest) => {
      if (!isAuthenticated) return { error: 'Not authenticated' };
      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return { error: 'Not authenticated' };
        const { data, error } = await unwrap(updateAppSettings(token, body));
        if (error) {
          toastManager.add({
            title: 'Failed to save server settings',
            description: error,
            type: 'error',
            timeout: 4000,
          });
          return { error };
        }
        toastManager.add({
          title: 'Server settings saved',
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
