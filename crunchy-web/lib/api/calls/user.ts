import { get } from '@/lib/api/client';
import type { AuthUser } from '@/types/api';

export type UserProfile = AuthUser & {
  created_at: string;
};

export const getUser = (token: string) =>
  get<UserProfile>(token, '/auth/me');
