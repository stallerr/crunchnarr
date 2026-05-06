'use client';

import Link from 'next/link';
import { DownloadProgressBar } from '@/components/downloads/download-progress-bar';
import { DownloadStatusBadge } from '@/components/downloads/download-status-badge';
import { formatEpisodeCode } from '@/lib/format';
import { proxyImageUrl } from '@/lib/image-helpers';
import { useDownloadProgress } from '@/hooks/use-download-progress';
import type { DownloadRow, DownloadStatus } from '@/types/downloads';

type ActiveDownloadsSectionProps = {
  downloads: DownloadRow[];
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

export function ActiveDownloadsSection({
  downloads,
}: ActiveDownloadsSectionProps) {
  const { getProgress } = useDownloadProgress();
  const active = downloads.slice(0, 5);

  return (
    <div className="rounded-xl border bg-card p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold">Active Downloads</h2>
        <Link href="/downloads" className="text-sm text-primary hover:underline">
          View all
        </Link>
      </div>

      {active.length === 0 ? (
        <p className="text-sm text-muted-foreground py-4">
          No active downloads
        </p>
      ) : (
        <div className="space-y-3">
          {active.map((download) => {
            const realtime = getProgress(download.id);
            const progress = realtime ? realtime.percent : parseProgress(download.progress_json);
            return (
              <div key={download.id} className="flex items-start gap-3">
                <div className="shrink-0 w-44 aspect-video rounded-md overflow-hidden bg-muted">
                  {download.thumbnail_url ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img
                      src={proxyImageUrl(download.thumbnail_url)}
                      alt=""
                      className="w-full h-full object-cover"
                    />
                  ) : (
                    <div className="w-full h-full flex items-center justify-center text-muted-foreground">
                      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>
                    </div>
                  )}
                </div>
                <div className="flex-1 min-w-0 space-y-1.5">
                  <div className="flex items-center gap-2">
                    <DownloadStatusBadge status={download.status as DownloadStatus} />
                    <span className="text-xs text-muted-foreground font-mono shrink-0">
                      {formatEpisodeCode(
                        download.season_number,
                        download.episode_number
                      )}
                    </span>
                    <span className="text-sm font-medium truncate">
                      {download.series_title} - {download.episode_title}
                    </span>
                  </div>
                  <DownloadProgressBar
                    progress={progress}
                    phase={realtime?.phase}
                    currentStep={realtime?.current_step}
                    totalSteps={realtime?.total_steps}
                    completedSegments={realtime?.completed_segments}
                    totalSegments={realtime?.total_segments}
                  />
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
