export type CRImage = {
  source: string;
  width: number;
  height: number;
  type?: string;
};

export type CRImages = {
  poster_tall?: CRImage[][];
  poster_wide?: CRImage[][];
  thumbnail?: CRImage[][];
};

export type CRSearchResult = {
  type: string;
  count: number;
  items: CRSearchItem[];
};

export type CRSearchItem = {
  id: string;
  title: string;
  slug_title: string;
  description: string;
  type: string;
  images: CRImages;
};

export type CRSeries = {
  id: string;
  title: string;
  slug_title: string;
  description: string;
  keywords: string[];
  season_count: number;
  episode_count: number;
  is_simulcast: boolean;
  is_mature: boolean;
  maturity_ratings: string[];
  content_provider: string;
  images: CRImages;
};

export type CRSeasonVersion = {
  audio_locale: string;
  guid: string;
  is_premium_only: boolean;
  original: boolean;
  variant: string;
  season_guid: string;
};

export type CRSeason = {
  id: string;
  title: string;
  slug_title: string;
  series_id: string;
  season_number: number;
  season_sequence_number: number;
  number_of_episodes: number;
  is_subbed: boolean;
  is_dubbed: boolean;
  is_simulcast: boolean;
  audio_locale: string;
  audio_locales: string[];
  subtitle_locales: string[];
  versions: CRSeasonVersion[];
};

export type CREpisodeVersion = {
  audio_locale: string;
  guid: string;
  media_guid: string;
  is_premium_only: boolean;
  original: boolean;
  variant: string;
  season_guid: string;
};

export type CREpisode = {
  id: string;
  title: string;
  slug_title: string;
  description: string;
  series_id: string;
  series_title: string;
  series_slug_title: string;
  season_id: string;
  season_title: string;
  season_number: number;
  season_sequence_number: number;
  episode: string;
  episode_number: number | null;
  sequence_number: number;
  duration_ms: number;
  is_premium_only: boolean;
  is_subbed: boolean;
  is_dubbed: boolean;
  is_mature: boolean;
  audio_locale: string;
  subtitle_locales: string[];
  versions: CREpisodeVersion[];
  streams_link: string;
  images: CRImages;
  episode_air_date: string | null;
  premium_available_date: string | null;
};

export type CRProfile = {
  username: string;
  email: string;
  avatar: string;
  preferred_communication_language: string;
  preferred_content_subtitle_language: string;
  preferred_content_audio_language: string;
};
