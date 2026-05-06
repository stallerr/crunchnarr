'use client';

import { useState } from 'react';
import { CheckCircle2Icon, XCircleIcon, LinkIcon, RefreshCwIcon, UnlinkIcon } from 'lucide-react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { UnlinkConfirmationDialog } from '@/components/account/unlink-confirmation-dialog';
import { useCrunchyrollStatus } from '@/hooks/use-crunchyroll';
import { useRouter } from 'next/navigation';

export function CrunchyrollSection() {
  const { isLinked, isLoading, profile, refetch } = useCrunchyrollStatus();
  const [unlinkOpen, setUnlinkOpen] = useState(false);
  const router = useRouter();

  if (isLoading) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Crunchyroll Account</CardTitle>
        </CardHeader>
        <CardPanel>
          <div className="h-16 rounded-xl bg-muted animate-pulse" />
        </CardPanel>
      </Card>
    );
  }

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>Crunchyroll Account</CardTitle>
          <CardDescription>
            Link your Crunchyroll account to search and download content.
          </CardDescription>
        </CardHeader>
        <CardPanel>
          {isLinked && profile ? (
            <div className="flex flex-col gap-4">
              <div className="flex items-center gap-3">
                {profile.avatar ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img
                    src={`https://static.crunchyroll.com/assets/avatar/170x170/${profile.avatar}`}
                    alt={profile.username}
                    className="size-10 rounded-full shrink-0 object-cover"
                  />
                ) : (
                  <CheckCircle2Icon className="size-5 text-green-500 shrink-0" />
                )}
                <div className="min-w-0">
                  <p className="text-sm font-medium">Connected</p>
                  <p className="text-xs text-muted-foreground truncate">{profile.email}</p>
                </div>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-sm">
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs text-muted-foreground">Username</span>
                  <span className="font-medium">{profile.username}</span>
                </div>
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs text-muted-foreground">Audio Language</span>
                  <span className="font-medium">
                    {profile.preferred_content_audio_language || '—'}
                  </span>
                </div>
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs text-muted-foreground">Subtitle Language</span>
                  <span className="font-medium">
                    {profile.preferred_content_subtitle_language || '—'}
                  </span>
                </div>
              </div>

              <div className="flex gap-2 pt-1">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => router.push('/link-crunchyroll')}
                >
                  <RefreshCwIcon />
                  Re-link
                </Button>
                <Button
                  variant="destructive-outline"
                  size="sm"
                  onClick={() => setUnlinkOpen(true)}
                >
                  <UnlinkIcon />
                  Unlink
                </Button>
              </div>
            </div>
          ) : (
            <div className="flex flex-col gap-4">
              <div className="flex items-center gap-3">
                <XCircleIcon className="size-5 text-muted-foreground shrink-0" />
                <div>
                  <p className="text-sm font-medium">Not connected</p>
                  <p className="text-xs text-muted-foreground">
                    Link your account to enable search and downloads
                  </p>
                </div>
              </div>

              <div>
                <Button size="sm" onClick={() => router.push('/link-crunchyroll')}>
                  <LinkIcon />
                  Link Account
                </Button>
              </div>
            </div>
          )}
        </CardPanel>
      </Card>

      <UnlinkConfirmationDialog
        open={unlinkOpen}
        onOpenChange={setUnlinkOpen}
        onUnlinked={refetch}
      />
    </>
  );
}
