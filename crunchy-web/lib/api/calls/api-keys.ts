import { get, post, del } from '@/lib/api/client';

export type ApiKeyItem = {
  id: string;
  name: string;
  key_prefix: string;
  created_at: string;
  last_used_at: string | null;
};

export type CreateApiKeyResponse = {
  id: string;
  name: string;
  /** Full key — returned exactly once at creation. */
  key: string;
  key_prefix: string;
  created_at: string;
};

export const listApiKeys = (token: string) =>
  get<ApiKeyItem[]>(token, '/api-keys');

export const createApiKey = (token: string, name: string) =>
  post<CreateApiKeyResponse>(token, '/api-keys', { name });

export const revokeApiKey = (token: string, id: string) =>
  del<void>(token, `/api-keys/${encodeURIComponent(id)}`);
