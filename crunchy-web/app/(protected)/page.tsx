'use client';

import {
  PagePanel,
  PageHeader,
  PageTitle,
  PageDescription,
} from '@/components/layout/page';
import { LinkBanner } from '@/components/crunchyroll/link-banner';
import { MetricCards } from '@/components/dashboard/metric-cards';
import { ActiveDownloadsSection } from '@/components/dashboard/active-downloads-section';
import { RecentActivity } from '@/components/dashboard/recent-activity';
import { useCrunchyrollStatus } from '@/hooks/use-crunchyroll';
import { useInfiniteDownloads, useDownloadCounts } from '@/hooks/use-downloads';

export default function HomePage() {
  const { isLinked, isLoading: isLinkLoading } = useCrunchyrollStatus();
  const { data: counts } = useDownloadCounts();
  const { items: activeDownloads } = useInfiniteDownloads('active');
  const { items: recentDownloads } = useInfiniteDownloads();

  return (
    <PagePanel>
      <PageHeader>
        <PageTitle>Dashboard</PageTitle>
        <PageDescription>
          Search, browse, and download content from Crunchyroll.
        </PageDescription>
      </PageHeader>

      {!isLinkLoading && !isLinked && (
        <div className="mb-6">
          <LinkBanner />
        </div>
      )}

      <div className="space-y-6">
        <MetricCards counts={counts} />

        <ActiveDownloadsSection downloads={activeDownloads} />

        <RecentActivity downloads={recentDownloads} />
      </div>
    </PagePanel>
  );
}
