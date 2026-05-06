'use client';

import { UserIcon } from 'lucide-react';
import {
  PagePanel,
  PageHeader,
  PageTitle,
  PageDescription,
} from '@/components/layout/page';
import { ProfileSection } from '@/components/account/profile-section';
import { AvatarPicker } from '@/components/account/avatar-picker';
import { CrunchyrollSection } from '@/components/account/crunchyroll-section';
import { useUser } from '@/hooks/use-user';

export default function AccountPage() {
  const { data: user, isLoading, error } = useUser();

  return (
    <PagePanel>
      <PageHeader>
        <div className="flex items-center gap-2">
          <UserIcon className="size-6 text-primary" />
          <PageTitle>Account</PageTitle>
        </div>
        <PageDescription>
          Manage your account profile and connected services.
        </PageDescription>
      </PageHeader>

      <div className="space-y-6">
        {isLoading ? (
          <div className="h-32 rounded-2xl border bg-card animate-pulse" />
        ) : error ? (
          <div className="flex flex-col items-center py-8 text-muted-foreground">
            <p className="text-sm">{error}</p>
          </div>
        ) : user ? (
          <>
            <ProfileSection user={user} />
            <AvatarPicker />
          </>
        ) : null}

        <CrunchyrollSection />
      </div>
    </PagePanel>
  );
}
