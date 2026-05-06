'use client';

import { useQuery } from '@/hooks/use-query';
import { getSeries, getSeasons, getEpisodes } from '@/lib/api/calls/content';
import type { CRSeries, CRSeason, CREpisode } from '@/types/crunchyroll';

export function useSeries(seriesId: string) {
  return useQuery<CRSeries>(
    (token) => getSeries(token, seriesId),
    [seriesId],
    { enabled: !!seriesId }
  );
}

export function useSeasons(seriesId: string) {
  return useQuery<CRSeason[]>(
    (token) => getSeasons(token, seriesId),
    [seriesId],
    { enabled: !!seriesId }
  );
}

export function useEpisodes(seasonId: string | null) {
  return useQuery<CREpisode[]>(
    (token) => getEpisodes(token, seasonId!),
    [seasonId],
    { enabled: !!seasonId }
  );
}
