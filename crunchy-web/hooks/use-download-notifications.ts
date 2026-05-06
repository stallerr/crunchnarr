'use client';

import { useRef, useCallback } from 'react';
import { useWebSocketSubscription } from '@/hooks/use-websocket';
import { toastManager } from '@/components/ui/toast';
import type { WsDownloadComplete, WsDownloadFailed } from '@/types/websocket';

const DEDUP_TTL_MS = 5_000;

export function useDownloadNotifications() {
  const seenIds = useRef(new Set<string>());

  const isDuplicate = useCallback((id: string): boolean => {
    if (seenIds.current.has(id)) return true;
    seenIds.current.add(id);
    setTimeout(() => {
      seenIds.current.delete(id);
    }, DEDUP_TTL_MS);
    return false;
  }, []);

  useWebSocketSubscription('download_complete', (data) => {
    const msg = data as WsDownloadComplete;
    if (isDuplicate(`complete:${msg.download_id}`)) return;

    toastManager.add({
      title: 'Download Complete',
      description: 'A download has finished successfully.',
      type: 'success',
      timeout: 5000,
    });
  });

  useWebSocketSubscription('download_failed', (data) => {
    const msg = data as WsDownloadFailed;
    if (isDuplicate(`failed:${msg.download_id}`)) return;

    toastManager.add({
      title: 'Download Failed',
      description: msg.error || 'An unknown error occurred.',
      type: 'error',
      timeout: 10000,
    });
  });
}
