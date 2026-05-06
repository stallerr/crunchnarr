'use client';

import { HardDriveIcon } from 'lucide-react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { formatBytes } from '@/lib/format';
import type { CacheSummary } from '@/lib/api/calls/cache';

type Props = {
  summary: CacheSummary;
};

const STORAGE_BAR_MAX_BYTES = 10 * 1024 * 1024 * 1024; // 10 GB reference scale

export function CacheSummaryCard({ summary }: Props) {
  const usedPercent = Math.min(
    100,
    (summary.total_size_bytes / STORAGE_BAR_MAX_BYTES) * 100
  );

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center gap-2">
          <HardDriveIcon className="size-5 text-primary" />
          <CardTitle>Cache Summary</CardTitle>
        </div>
        <CardDescription>
          Cached segment data from in-progress and completed downloads.
        </CardDescription>
      </CardHeader>
      <CardPanel>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-5">
          <div className="flex flex-col gap-1">
            <span className="text-xs text-muted-foreground">Total Size</span>
            <span className="text-2xl font-semibold">
              {formatBytes(summary.total_size_bytes)}
            </span>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-xs text-muted-foreground">Cache Entries</span>
            <span className="text-2xl font-semibold">{summary.entry_count}</span>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-xs text-muted-foreground">Retention</span>
            <span className="text-2xl font-semibold">{summary.retention_days}d</span>
          </div>
        </div>

        <div className="flex flex-col gap-1.5">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>Storage used</span>
            <span>{usedPercent.toFixed(1)}%</span>
          </div>
          <div className="h-2 w-full rounded-full bg-muted overflow-hidden">
            <div
              className="h-full rounded-full bg-primary transition-all duration-500"
              style={{ width: `${usedPercent}%` }}
            />
          </div>
          <p className="text-xs text-muted-foreground">
            {formatBytes(summary.total_size_bytes)} of{' '}
            {formatBytes(STORAGE_BAR_MAX_BYTES)} reference scale
          </p>
        </div>
      </CardPanel>
    </Card>
  );
}
