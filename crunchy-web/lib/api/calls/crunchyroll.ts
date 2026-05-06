import { get, post } from '@/lib/api/client';
import type { CRProfile } from '@/types/crunchyroll';

type CrLoginRequest = {
  username?: string;
  password?: string;
  refresh_token?: string;
};

type CrLoginResponse = {
  status: string;
  account_id?: string;
};

type StatusResponse = {
  status: string;
};

export const loginCrunchyroll = (token: string, data: CrLoginRequest) =>
  post<CrLoginResponse>(token, '/crunchyroll/login', data);

export const logoutCrunchyroll = (token: string) =>
  post<StatusResponse>(token, '/crunchyroll/logout');

export const getCrunchyrollProfile = (token: string) =>
  get<CRProfile>(token, '/crunchyroll/whoami');
