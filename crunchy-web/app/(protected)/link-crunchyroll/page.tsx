'use client';

import { useRouter } from 'next/navigation';
import { PagePanel, PageHeader, PageTitle, PageDescription } from '@/components/layout/page';
import { LinkAccountCard } from '@/components/crunchyroll/link-account-card';

export default function LinkCrunchyrollPage() {
  const router = useRouter();

  return (
    <PagePanel>
      <PageHeader className="text-center">
        <PageTitle>Connect Crunchyroll</PageTitle>
        <PageDescription>
          Link your Crunchyroll account to start browsing and downloading.
        </PageDescription>
      </PageHeader>

      <LinkAccountCard onSuccess={() => router.push('/')} />
    </PagePanel>
  );
}
