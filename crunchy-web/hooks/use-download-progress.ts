'use client';

import { useCallback, useState } from 'react';
import { useWebSocketSubscription } from '@/hooks/use-websocket';
import type {
  WsDownloadProgress,
  WsDownloadComplete,
  WsDownloadFailed,
} from '@/types/websocket';

export interface RealtimeProgress {
  percent: number;
  phase: string;
  speed_bps: number;
  eta_secs: number | null;
  downloaded_bytes: number;
  total_bytes: number | null;
  current_step?: number;
  total_steps?: number;
  completed_segments?: number;
  total_segments?: number;
}

export function useDownloadProgress(): {
  getProgress: (downloadId: string) => RealtimeProgress | null;
} {
  const [progressMap, setProgressMap] = useState<Map<string, RealtimeProgress>>(
    () => new Map()
  );

  useWebSocketSubscription('download_progress', (data) => {
    const msg = data as WsDownloadProgress;
    setProgressMap((prev) => {
      const next = new Map(prev);
      next.set(msg.download_id, {
        percent: msg.percent,
        phase: msg.phase,
        speed_bps: msg.speed_bps,
        eta_secs: msg.eta_secs,
        downloaded_bytes: msg.downloaded_bytes,
        total_bytes: msg.total_bytes,
        current_step: msg.current_step,
        total_steps: msg.total_steps,
        completed_segments: msg.completed_segments,
        total_segments: msg.total_segments,
      });
      return next;
    });
  });

  useWebSocketSubscription('download_complete', (data) => {
    const msg = data as WsDownloadComplete;
    setProgressMap((prev) => {
      if (!prev.has(msg.download_id)) return prev;
      const next = new Map(prev);
      next.delete(msg.download_id);
      return next;
    });
  });

  useWebSocketSubscription('download_failed', (data) => {
    const msg = data as WsDownloadFailed;
    setProgressMap((prev) => {
      if (!prev.has(msg.download_id)) return prev;
      const next = new Map(prev);
      next.delete(msg.download_id);
      return next;
    });
  });

  const getProgress = useCallback(
    (downloadId: string): RealtimeProgress | null => {
      return progressMap.get(downloadId) ?? null;
    },
    [progressMap]
  );

  return { getProgress };
}
