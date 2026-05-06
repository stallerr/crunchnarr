import { get } from '@/lib/api/client';
import type { CRSearchResult } from '@/types/crunchyroll';

export const searchContent = (token: string, query: string, limit = 10) =>
  get<CRSearchResult[]>(token, '/search', { params: { q: query, limit } });
