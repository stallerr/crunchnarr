'use client';

import Link from 'next/link';
import { CalendarIcon, ClockIcon, CrownIcon } from 'lucide-react';
import type { CREpisode } from '@/types/crunchyroll';

type EpisodeMetadataProps = {
  episode: CREpisode;
};

function formatDuration(ms: number): string {
  const minutes = Math.round(ms / 60000);
  return `${minutes} min`;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

export function EpisodeMetadata({ episode }: EpisodeMetadataProps) {
  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Link
          href={`/series/${episode.series_id}`}
          className="text-primary hover:underline"
        >
          {episode.series_title}
        </Link>
      </div>

      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <span>Season {episode.season_number}</span>
        <span className="text-muted-foreground/40">|</span>
        <span>Episode {episode.episode}</span>
      </div>

      <h1 className="text-2xl font-semibold font-display">{episode.title}</h1>

      <div className="flex items-center gap-4 text-sm text-muted-foreground">
        <span className="flex items-center gap-1.5">
          <ClockIcon className="size-3.5" />
          {formatDuration(episode.duration_ms)}
        </span>
        {episode.episode_air_date && (
          <span className="flex items-center gap-1.5">
            <CalendarIcon className="size-3.5" />
            {formatDate(episode.episode_air_date)}
          </span>
        )}
        {episode.is_premium_only && (
          <span className="flex items-center gap-1.5 text-yellow-500">
            <CrownIcon className="size-3.5" />
            Premium
          </span>
        )}
      </div>

      {episode.description && (
        <p className="text-sm text-muted-foreground leading-relaxed">
          {episode.description}
        </p>
      )}
    </div>
  );
}
