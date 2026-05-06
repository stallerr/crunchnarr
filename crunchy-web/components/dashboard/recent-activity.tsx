'use client';

import Link from 'next/link';
import { DownloadStatusBadge } from '@/components/downloads/download-status-badge';
import { formatRelativeTime, formatEpisodeCode } from '@/lib/format';
import type { DownloadRow } from '@/types/downloads';

type RecentActivityProps = {
  downloads: DownloadRow[];
};

export function RecentActivity({ downloads }: RecentActivityProps) {
  const recent = downloads
    .filter((d) => d.status === 'completed' || d.status === 'failed')
    .sort(
      (a, b) =>
        new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
    )
    .slice(0, 10);

  return (
    <div className="rounded-xl border bg-card p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold">Recent Activity</h2>
        <Link href="/downloads" className="text-sm text-primary hover:underline">
          View all downloads
        </Link>
      </div>

      {recent.length === 0 ? (
        <p className="text-sm text-muted-foreground py-4">
          No recent activity
        </p>
      ) : (
        <div className="space-y-2">
          {recent.map((download) => (
            <div
              key={download.id}
              className="flex items-center gap-3 py-1.5"
            >
              <DownloadStatusBadge status={download.status} />
              <div className="flex items-center gap-2 min-w-0 flex-1">
                <span className="text-xs text-muted-foreground font-mono shrink-0">
                  {formatEpisodeCode(
                    download.season_number,
                    download.episode_number
                  )}
                </span>
                <span className="text-sm truncate">
                  {download.series_title} - {download.episode_title}
                </span>
              </div>
              <span className="text-xs text-muted-foreground shrink-0">
                {formatRelativeTime(download.updated_at)}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
