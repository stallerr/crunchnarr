'use client';

import { useRef, useEffect } from 'react';
import Link from 'next/link';
import { Loader2 } from 'lucide-react';
import { DownloadStatusBadge } from './download-status-badge';
import { DownloadProgressBar } from './download-progress-bar';
import { DownloadSpeedIndicator } from './download-speed-indicator';
import { formatRelativeTime, formatEpisodeCode } from '@/lib/format';
import { proxyImageUrl } from '@/lib/image-helpers';
import { useDownloadProgress } from '@/hooks/use-download-progress';
import type { RealtimeProgress } from '@/hooks/use-download-progress';
import { useDensity } from '@/components/providers/density-provider';
import type { DownloadRow, DownloadStatus } from '@/types/downloads';

type DownloadsTableProps = {
  downloads: DownloadRow[];
  renderActions?: (download: DownloadRow) => React.ReactNode;
  hasMore?: boolean;
  isLoadingMore?: boolean;
  onLoadMore?: () => void;
};

function parseProgress(json: string): number | null {
  try {
    const parsed = JSON.parse(json);
    if (typeof parsed?.percentage === 'number') return parsed.percentage;
    return null;
  } catch {
    return null;
  }
}

function parseQuality(json: string): string {
  try {
    const parsed = JSON.parse(json);
    return parsed?.videoQuality ?? 'best';
  } catch {
    return 'best';
  }
}

export function DownloadsTable({
  downloads,
  renderActions,
  hasMore,
  isLoadingMore,
  onLoadMore,
}: DownloadsTableProps) {
  const { getProgress } = useDownloadProgress();
  const { density } = useDensity();
  const sentinelRef = useRef<HTMLDivElement>(null);

  // IntersectionObserver to trigger loadMore when sentinel is visible
  useEffect(() => {
    if (!hasMore || !onLoadMore) return;

    const sentinel = sentinelRef.current;
    if (!sentinel) return;

    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting && !isLoadingMore) {
          onLoadMore();
        }
      },
      { threshold: 0.1 }
    );

    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [hasMore, isLoadingMore, onLoadMore]);

  if (downloads.length === 0) {
    return (
      <div className="flex flex-col items-center py-16 text-muted-foreground">
        <p className="text-sm">No downloads found</p>
      </div>
    );
  }

  return (
    <div className={density === 'compact' ? 'space-y-1' : 'space-y-2'}>
      {downloads.map((download) => (
        <DownloadItem
          key={download.id}
          download={download}
          renderActions={renderActions}
          realtimeProgress={getProgress(download.id)}
          compact={density === 'compact'}
        />
      ))}

      {/* Scroll sentinel for infinite loading */}
      {hasMore && (
        <div ref={sentinelRef} className="flex justify-center py-4">
          {isLoadingMore && (
            <Loader2 className="size-5 animate-spin text-muted-foreground" />
          )}
        </div>
      )}
    </div>
  );
}

function DownloadItem({
  download,
  renderActions,
  realtimeProgress,
  compact,
}: {
  download: DownloadRow;
  renderActions?: (download: DownloadRow) => React.ReactNode;
  realtimeProgress: RealtimeProgress | null;
  compact: boolean;
}) {
  const staticProgress = parseProgress(download.progress_json);
  const quality = parseQuality(download.options_json);
  const isActive = download.status === 'active';

  // Prefer real-time WebSocket data over static progress_json
  const progress = realtimeProgress ? realtimeProgress.percent : staticProgress;
  const phase = realtimeProgress?.phase ?? undefined;

  const episodeHref = `/episodes/${download.episode_id}`;

  return (
    <div className={compact ? "flex items-center gap-3 p-2 rounded-lg border bg-card" : "flex items-center gap-4 p-3 rounded-xl border bg-card"}>
      <Link
        href={episodeHref}
        className={compact ? "shrink-0 w-32 aspect-video rounded-md overflow-hidden bg-muted group" : "shrink-0 w-48 aspect-video rounded-md overflow-hidden bg-muted group"}
        title={`Open ${download.episode_title ?? 'episode'}`}
      >
        {download.thumbnail_url ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={proxyImageUrl(download.thumbnail_url)}
            alt=""
            className="w-full h-full object-cover transition-transform duration-300 group-hover:scale-105"
          />
        ) : (
          <div className="w-full h-full flex items-center justify-center text-muted-foreground">
            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>
          </div>
        )}
      </Link>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <DownloadStatusBadge status={download.status as DownloadStatus} />
          <span className="text-xs text-muted-foreground font-mono">
            {formatEpisodeCode(download.season_number, download.episode_number)}
          </span>
          <span className="text-xs px-1.5 py-0.5 rounded bg-secondary text-secondary-foreground">
            {quality}
          </span>
        </div>
        <Link
          href={episodeHref}
          className="block text-sm font-medium line-clamp-1 mt-0.5 hover:underline"
        >
          {download.series_title} - {download.episode_title}
        </Link>
        {isActive && (
          <>
            <DownloadProgressBar
              progress={progress}
              phase={phase}
              currentStep={realtimeProgress?.current_step}
              totalSteps={realtimeProgress?.total_steps}
              completedSegments={realtimeProgress?.completed_segments}
              totalSegments={realtimeProgress?.total_segments}
              className="mt-1.5"
            />
            {realtimeProgress && (
              <DownloadSpeedIndicator
                speed_bps={realtimeProgress.speed_bps}
                eta_secs={realtimeProgress.eta_secs}
                className="mt-0.5"
              />
            )}
          </>
        )}
        {download.error && (
          <p className="text-xs text-red-500 mt-1 line-clamp-1">
            {download.error}
          </p>
        )}
      </div>

      <div className="shrink-0 text-xs text-muted-foreground">
        {formatRelativeTime(download.created_at)}
      </div>

      {renderActions && (
        <div className="shrink-0 flex items-center gap-1">
          {renderActions(download)}
        </div>
      )}
    </div>
  );
}
