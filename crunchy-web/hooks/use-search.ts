'use client';

import { useState, useCallback, useRef } from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { searchContent } from '@/lib/api/calls/search';
import type { CRSearchResult } from '@/types/crunchyroll';

export function useSearch() {
  const { getToken, isAuthenticated } = useAuthToken();
  const [results, setResults] = useState<CRSearchResult[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const search = useCallback(
    (query: string, limit = 10) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);

      if (!query.trim()) {
        setResults([]);
        setError(null);
        setIsLoading(false);
        return;
      }

      setIsLoading(true);
      setResults([]);

      debounceRef.current = setTimeout(async () => {
        if (!isAuthenticated) {
          setError('Not authenticated');
          setIsLoading(false);
          return;
        }

        try {
          const token = await getToken();
          if (!token) {
            setError('Not authenticated');
            setIsLoading(false);
            return;
          }
          const result = await searchContent(token, query, limit);
          if (result.success) {
            setResults(result.data);
            setError(null);
          } else {
            setError('Search failed');
          }
        } catch {
          setError('An unexpected error occurred');
        } finally {
          setIsLoading(false);
        }
      }, 300);
    },
    [getToken, isAuthenticated]
  );

  const clear = useCallback(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    setResults([]);
    setError(null);
    setIsLoading(false);
  }, []);

  return { results, isLoading, error, search, clear };
}
