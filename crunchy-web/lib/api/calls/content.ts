import { get } from '@/lib/api/client';
import type { CRSeries, CRSeason, CREpisode } from '@/types/crunchyroll';

export const getSeries = (token: string, id: string) =>
  get<CRSeries>(token, `/series/${id}`);

export const getSeasons = (token: string, seriesId: string) =>
  get<CRSeason[]>(token, `/series/${seriesId}/seasons`);

export const getEpisodes = (token: string, seasonId: string) =>
  get<CREpisode[]>(token, `/seasons/${seasonId}/episodes`);

export const getEpisode = (token: string, id: string) =>
  get<CREpisode>(token, `/episodes/${id}`);
