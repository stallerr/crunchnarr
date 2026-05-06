'use client';

import { useQuery } from '@/hooks/use-query';
import { getUser } from '@/lib/api/calls/user';
import type { UserProfile } from '@/lib/api/calls/user';

export function useUser() {
  return useQuery<UserProfile>((token) => getUser(token), []);
}
