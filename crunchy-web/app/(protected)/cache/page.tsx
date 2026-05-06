'use client';

import { useState } from 'react';
import { HardDriveIcon, Trash2Icon } from 'lucide-react';
import {
  PagePanel,
  PageHeader,
  PageTitle,
  PageDescription,
} from '@/components/layout/page';
import { Button } from '@/components/ui/button';
import { CacheSummaryCard } from '@/components/cache/cache-summary-card';
import { CacheEntriesTable } from '@/components/cache/cache-entries-table';
import { CleanCacheDialog } from '@/components/cache/clean-cache-dialog';
import { useCache, useCleanCache } from '@/hooks/use-cache';

export default function CachePage() {
  const { data: cache, isLoading, error, refetch } = useCache();
  const { execute: cleanCache, isLoading: isCleaning } = useCleanCache();
  const [cleanDialogOpen, setCleanDialogOpen] = useState(false);

  const handleClean = async () => {
    const { error } = await cleanCache();
    if (!error) {
      setCleanDialogOpen(false);
      refetch();
    }
  };

  return (
    <PagePanel>
      <PageHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <HardDriveIcon className="size-6 text-primary" />
            <PageTitle>Download Cache</PageTitle>
          </div>
          {cache && cache.entry_count > 0 && (
            <Button
              variant="destructive-outline"
              size="sm"
              onClick={() => setCleanDialogOpen(true)}
              disabled={isCleaning}
            >
              <Trash2Icon />
              Clean All
            </Button>
          )}
        </div>
        <PageDescription>
          View and manage cached segment data from downloads.
        </PageDescription>
      </PageHeader>

      {isLoading ? (
        <div className="space-y-4">
          <div className="h-40 rounded-2xl border bg-card animate-pulse" />
          <div className="h-64 rounded-2xl border bg-card animate-pulse" />
        </div>
      ) : error ? (
        <div className="flex flex-col items-center py-16 text-muted-foreground">
          <p className="text-sm">{error}</p>
        </div>
      ) : cache ? (
        <div className="space-y-6">
          <CacheSummaryCard summary={cache} />

          <div>
            <h2 className="text-base font-semibold mb-3">Cache Entries</h2>
            <CacheEntriesTable entries={cache.entries} />
          </div>
        </div>
      ) : null}

      {cache && (
        <CleanCacheDialog
          open={cleanDialogOpen}
          onOpenChange={setCleanDialogOpen}
          entryCount={cache.entry_count}
          totalSizeBytes={cache.total_size_bytes}
          isLoading={isCleaning}
          onConfirm={handleClean}
        />
      )}
    </PagePanel>
  );
}
