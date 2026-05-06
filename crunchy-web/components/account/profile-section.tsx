'use client';

import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { formatRelativeTime } from '@/lib/format';
import { getAvatarById } from '@/lib/avatars';
import { useAvatar } from '@/hooks/use-avatar';
import type { UserProfile } from '@/lib/api/calls/user';

type Props = {
  user: UserProfile;
};

export function ProfileSection({ user }: Props) {
  const { avatarId } = useAvatar();
  const avatar = getAvatarById(avatarId);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Profile</CardTitle>
        <CardDescription>Your crunchy-web account details.</CardDescription>
      </CardHeader>
      <CardPanel>
        <div className="flex items-center gap-4">
          <div className="relative size-16 shrink-0 overflow-hidden rounded-full">
            <div className="absolute inset-0 flex items-center justify-center">
              <div className="scale-[1.6] transform">{avatar.svg}</div>
            </div>
          </div>
          <div className="flex flex-col gap-1 min-w-0">
            <p className="text-base font-semibold truncate">{user.username}</p>
            <p className="text-sm text-muted-foreground truncate">{user.email}</p>
            {user.created_at && (
              <p className="text-xs text-muted-foreground">
                Joined {formatRelativeTime(user.created_at)}
              </p>
            )}
          </div>
        </div>
      </CardPanel>
    </Card>
  );
}
