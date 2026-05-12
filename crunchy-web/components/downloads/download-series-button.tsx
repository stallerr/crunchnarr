'use client';

import { useState, useCallback } from 'react';
import { DownloadIcon, LoaderCircleIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { startDownload } from '@/lib/api/calls/downloads';
import { unwrap } from '@/lib/api/client';
import { toastManager } from '@/components/ui/toast';
import { DEFAULT_DOWNLOAD_OPTIONS } from '@/types/download-options';

type DownloadSeriesButtonProps = {
  seriesId: string;
  seasonId: string;
  episodeCount: number;
};

export function DownloadSeriesButton({
  seriesId,
  episodeCount,
}: DownloadSeriesButtonProps) {
  const { getToken, isAuthenticated } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const handleDownload = useCallback(
    async (force = false) => {
      if (!isAuthenticated) return;

      setIsLoading(true);
      try {
        const token = await getToken();
        if (!token) return;

        const url = `https://www.crunchyroll.com/series/${seriesId}`;
        const { data, error } = await unwrap(
          startDownload(token, url, DEFAULT_DOWNLOAD_OPTIONS, force)
        );

        if (error || !data) {
          toastManager.add({
            type: 'error',
            title: 'Download failed',
            description: error ?? 'An unexpected error occurred',
          });
          return;
        }

        const started = data.filter((d) => d.status === 'pending').length;
        const skipped = data.filter((d) => d.status === 'skipped').length;

        if (started > 0 && skipped === 0) {
          toastManager.add({
            type: 'success',
            title: 'Season download started',
            description: `${started} episode${started !== 1 ? 's' : ''} queued`,
          });
        } else if (started === 0 && skipped > 0) {
          toastManager.add({
            type: 'info',
            title: 'Nothing to download',
            description: `${skipped} episode${skipped !== 1 ? 's' : ''} already downloaded`,
            timeout: 8000,
            actionProps: {
              children: 'Re-download all',
              onClick: () => {
                void handleDownload(true);
              },
            },
          });
        } else {
          toastManager.add({
            type: 'success',
            title: 'Season download started',
            description: `${started} new, ${skipped} skipped (already downloaded)`,
            timeout: 8000,
            actionProps: {
              children: 'Re-download all',
              onClick: () => {
                void handleDownload(true);
              },
            },
          });
        }
      } catch {
        toastManager.add({
          type: 'error',
          title: 'Download failed',
          description: 'An unexpected error occurred',
        });
      } finally {
        setIsLoading(false);
      }
    },
    [getToken, isAuthenticated, seriesId]
  );

  return (
    <Button variant="outline" size="sm" onClick={() => void handleDownload()} disabled={isLoading}>
      {isLoading ? (
        <LoaderCircleIcon className="animate-spin" />
      ) : (
        <DownloadIcon />
      )}
      Download Season ({episodeCount} ep{episodeCount !== 1 ? 's' : ''})
    </Button>
  );
}
