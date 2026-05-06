'use client';

import { useState } from 'react';
import { DownloadIcon } from 'lucide-react';
import {
  PagePanel,
  PageHeader,
  PageTitle,
  PageDescription,
} from '@/components/layout/page';
import { DownloadsTable } from '@/components/downloads/downloads-table';
import { DownloadActions } from '@/components/downloads/download-actions';
import { useInfiniteDownloads, useDownloadCounts } from '@/hooks/use-downloads';
import { cn } from '@/lib/utils';


const TABS = [
  { key: 'all', label: 'All' },
  { key: 'active', label: 'Active' },
  { key: 'completed', label: 'Completed' },
  { key: 'failed', label: 'Failed' },
  { key: 'cancelled', label: 'Cancelled' },
] as const;

type TabKey = (typeof TABS)[number]['key'];

// Map tab key to the status query param (undefined = no filter)
function tabToStatus(tab: TabKey): 'active' | 'completed' | 'failed' | 'cancelled' | undefined {
  if (tab === 'all') return undefined;
  return tab;
}

export default function DownloadsPage() {
  const [activeTab, setActiveTab] = useState<TabKey>('all');
  const { items, isLoading, isLoadingMore, error, hasMore, loadMore, refetch } =
    useInfiniteDownloads(tabToStatus(activeTab));
  const { data: counts } = useDownloadCounts();

  return (
    <PagePanel>
      <PageHeader>
        <div className="flex items-center gap-2">
          <DownloadIcon className="size-6 text-primary" />
          <PageTitle>Downloads</PageTitle>
        </div>
        <PageDescription>
          Manage your active and completed downloads
        </PageDescription>
      </PageHeader>

      {/* Tabs */}
      <div className="flex gap-1 mb-6 border-b">
        {TABS.map((tab) => {
          const count = counts?.[tab.key] ?? 0;

          return (
            <button
              key={tab.key}
              type="button"
              onClick={() => setActiveTab(tab.key)}
              className={cn(
                'px-3 py-2 text-sm font-medium transition-colors border-b-2 -mb-px',
                activeTab === tab.key
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              )}
            >
              {tab.label}
              {count > 0 && (
                <span className="ml-1.5 text-xs bg-muted px-1.5 py-0.5 rounded-full">
                  {count}
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* Content */}
      {isLoading ? (
        <div className="space-y-2">
          {Array.from({ length: 4 }).map((_, i) => (
            <div
              key={i}
              className="h-16 rounded-xl border bg-card animate-pulse"
            />
          ))}
        </div>
      ) : error ? (
        <div className="flex flex-col items-center py-16 text-muted-foreground">
          <p className="text-sm">{error}</p>
        </div>
      ) : (
        <DownloadsTable
          downloads={items}
          hasMore={hasMore}
          isLoadingMore={isLoadingMore}
          onLoadMore={loadMore}
          renderActions={(download) => (
            <DownloadActions
              download={download}
              onActionComplete={refetch}
            />
          )}
        />
      )}
    </PagePanel>
  );
}
