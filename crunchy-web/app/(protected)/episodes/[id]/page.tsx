'use client';

import { useParams } from 'next/navigation';
import Link from 'next/link';
import {
  ArrowLeftIcon,
  CheckCircle2Icon,
  CircleCheckBigIcon,
} from 'lucide-react';
import { PagePanel } from '@/components/layout/page';
import { Button } from '@/components/ui/button';
import { CRImage } from '@/components/ui/cr-image';
import { EpisodeMetadata } from '@/components/episode/episode-metadata';
import { AudioSubtitleBadges } from '@/components/episode/audio-subtitle-badges';
import { DownloadButton } from '@/components/downloads/download-button';
import { useQuery } from '@/hooks/use-query';
import { useDownloadedEpisodes, useMarkManual } from '@/hooks/use-downloads';
import { getEpisode } from '@/lib/api/calls/content';
import type { CREpisode } from '@/types/crunchyroll';

export default function EpisodeDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { data: episode, isLoading, error } = useQuery<CREpisode>(
    (token) => getEpisode(token, id),
    [id],
    { enabled: !!id }
  );
  const { completedIds, manualIds, refetch: refetchDownloaded } = useDownloadedEpisodes();
  const { mark, unmark, isLoading: marking } = useMarkManual();
  const isDownloaded = completedIds.has(id);
  const isMarked = manualIds.has(id);

  const handleMark = async () => {
    if (!episode) return;
    const { error } = await mark({
      episode_id: episode.id,
      series_title: episode.series_title,
      episode_title: episode.title,
      season_number: episode.season_number,
      episode_number: episode.episode_number ?? undefined,
      thumbnail_url: episode.images.thumbnail?.[0]?.[0]?.source ?? null,
    });
    if (!error) refetchDownloaded();
  };

  const handleUnmark = async () => {
    if (!episode) return;
    const { error } = await unmark(episode.id);
    if (!error) refetchDownloaded();
  };

  if (isLoading) {
    return (
      <PagePanel>
        <div className="animate-pulse space-y-4">
          <div className="aspect-video w-full max-w-2xl bg-muted rounded-xl" />
          <div className="h-6 w-48 bg-muted rounded" />
          <div className="h-8 w-64 bg-muted rounded" />
          <div className="h-4 w-full bg-muted rounded" />
        </div>
      </PagePanel>
    );
  }

  if (error || !episode) {
    return (
      <PagePanel>
        <div className="flex flex-col items-center py-16 text-muted-foreground">
          <p className="text-sm">{error ?? 'Episode not found'}</p>
        </div>
      </PagePanel>
    );
  }

  return (
    <PagePanel>
      <Link
        href={`/series/${episode.series_id}`}
        className="inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors mb-4"
      >
        <ArrowLeftIcon className="size-4" />
        Back to {episode.series_title}
      </Link>

      {/* Thumbnail */}
      <div className="aspect-video w-full max-w-2xl rounded-xl overflow-hidden mb-6">
        <CRImage
          images={episode.images}
          type="thumbnail"
          preferredWidth={800}
          alt={episode.title}
          className="w-full h-full object-cover"
        />
      </div>

      {/* Metadata */}
      <EpisodeMetadata episode={episode} />

      {/* Actions */}
      <div className="flex items-center gap-3 mt-6 flex-wrap">
        <DownloadButton episode={episode} />
        {isDownloaded ? (
          <span
            className="inline-flex items-center gap-1.5 text-sm px-2.5 py-1 rounded-md bg-emerald-500/15 text-emerald-500"
            title="You have downloaded this episode"
          >
            <CheckCircle2Icon className="size-4" />
            Downloaded
          </span>
        ) : isMarked ? (
          <>
            <span
              className="inline-flex items-center gap-1.5 text-sm px-2.5 py-1 rounded-md bg-amber-500/15 text-amber-500"
              title="You marked this episode as downloaded"
            >
              <CircleCheckBigIcon className="size-4" />
              Marked
            </span>
            <Button variant="ghost" size="sm" onClick={handleUnmark} disabled={marking}>
              Unmark
            </Button>
          </>
        ) : (
          <Button variant="outline" size="sm" onClick={handleMark} disabled={marking}>
            <CircleCheckBigIcon />
            Mark as already downloaded
          </Button>
        )}
      </div>

      {/* Audio/Subtitles */}
      <div className="mt-8 pt-6 border-t">
        <AudioSubtitleBadges
          audioLocale={episode.audio_locale}
          subtitleLocales={episode.subtitle_locales}
        />
      </div>

      {/* Version variants */}
      {episode.versions.length > 1 && (
        <div className="mt-6 pt-6 border-t">
          <h3 className="text-sm font-medium mb-2">Other Versions</h3>
          <div className="flex flex-wrap gap-1.5">
            {episode.versions
              .filter((v) => v.guid !== episode.id)
              .map((version) => (
                <Link
                  key={version.guid}
                  href={`/episodes/${version.guid}`}
                  className="px-2 py-1 rounded-md bg-secondary text-secondary-foreground text-xs hover:bg-secondary/80 transition-colors"
                >
                  {version.audio_locale}
                  {version.original && ' (Original)'}
                </Link>
              ))}
          </div>
        </div>
      )}
    </PagePanel>
  );
}
