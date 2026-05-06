'use client';

import { useState } from 'react';
import Link from 'next/link';
import {
  CheckCircle2Icon,
  CircleCheckBigIcon,
  CrownIcon,
  MoreVerticalIcon,
} from 'lucide-react';
import { CRImage } from '@/components/ui/cr-image';
import { Button } from '@/components/ui/button';
import {
  Popover,
  PopoverTrigger,
  PopoverContent,
  PopoverClose,
} from '@/components/ui/popover';
import { DownloadButton } from '@/components/downloads/download-button';
import { useMarkManual } from '@/hooks/use-downloads';
import { getLanguageName } from '@/lib/languages';
import type { CREpisode } from '@/types/crunchyroll';

type EpisodeListItemProps = {
  episode: CREpisode;
  /** Has a real (non-manual) completed download. */
  isDownloaded?: boolean;
  /** User has manually marked this episode as downloaded. */
  isMarked?: boolean;
  /** Refetch the downloaded-set after a manual mark/unmark. */
  onChanged?: () => void;
};

function formatDuration(ms: number): string {
  const minutes = Math.round(ms / 60000);
  return `${minutes} min`;
}

function thumbnailFor(episode: CREpisode): string | null {
  const variants = episode.images.thumbnail?.[0];
  if (!variants?.length) return null;
  // Pick the variant nearest to ~320px wide.
  return variants
    .slice()
    .sort((a, b) => Math.abs(a.width - 320) - Math.abs(b.width - 320))[0]
    .source;
}

export function EpisodeListItem({
  episode,
  isDownloaded = false,
  isMarked = false,
  onChanged,
}: EpisodeListItemProps) {
  const { mark, unmark, isLoading } = useMarkManual();
  const [open, setOpen] = useState(false);

  const handleMark = async () => {
    setOpen(false);
    const { error } = await mark({
      episode_id: episode.id,
      series_title: episode.series_title,
      episode_title: episode.title,
      season_number: episode.season_number,
      episode_number: episode.episode_number ?? undefined,
      thumbnail_url: thumbnailFor(episode),
    });
    if (!error) onChanged?.();
  };

  const handleUnmark = async () => {
    setOpen(false);
    const { error } = await unmark(episode.id);
    if (!error) onChanged?.();
  };

  return (
    <div className="flex gap-4 p-3 rounded-xl border bg-card hover:border-primary/30 transition-colors group">
      <Link
        href={`/episodes/${episode.id}`}
        className="shrink-0 w-40 aspect-video rounded-lg overflow-hidden"
      >
        <CRImage
          images={episode.images}
          type="thumbnail"
          preferredWidth={320}
          alt={episode.title}
          className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
        />
      </Link>

      <div className="flex-1 min-w-0">
        <Link href={`/episodes/${episode.id}`} className="block">
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground font-mono">
              E{episode.episode}
            </span>
            {episode.is_premium_only && (
              <CrownIcon className="size-3.5 text-yellow-500" />
            )}
            {isDownloaded && (
              <span
                className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-500"
                title="You have downloaded this episode"
              >
                <CheckCircle2Icon className="size-3" />
                Downloaded
              </span>
            )}
            {isMarked && !isDownloaded && (
              <span
                className="inline-flex items-center gap-1 text-xs px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-500"
                title="You marked this episode as downloaded"
              >
                <CircleCheckBigIcon className="size-3" />
                Marked
              </span>
            )}
          </div>
          <h4 className="font-medium text-sm line-clamp-1 mt-0.5">
            {episode.title}
          </h4>
          <p className="text-xs text-muted-foreground line-clamp-2 mt-1">
            {episode.description}
          </p>
        </Link>

        <div className="flex items-center gap-2 mt-2 flex-wrap">
          <span className="text-xs text-muted-foreground">
            {formatDuration(episode.duration_ms)}
          </span>
          {(episode.versions?.length ? episode.versions.map((v) => v.audio_locale) : [episode.audio_locale]).map((locale) => (
            <span
              key={locale}
              className="text-xs px-1.5 py-0.5 rounded bg-secondary text-secondary-foreground"
            >
              {getLanguageName(locale)}
            </span>
          ))}
        </div>
      </div>

      <div className="shrink-0 self-center flex items-center gap-1">
        <DownloadButton
          episode={episode}
          variant="ghost"
          size="icon-sm"
          showLabel={false}
        />
        <Popover open={open} onOpenChange={setOpen}>
          <PopoverTrigger
            render={(props) => (
              <Button
                {...props}
                variant="ghost"
                size="icon-sm"
                aria-label="More options"
              >
                <MoreVerticalIcon />
              </Button>
            )}
          />
          <PopoverContent className="p-1 min-w-48 flex flex-col" align="end">
            {isMarked && !isDownloaded ? (
              <PopoverClose
                render={(props) => (
                  <button
                    {...props}
                    onClick={handleUnmark}
                    disabled={isLoading}
                    className="flex items-center gap-2 px-2 py-1.5 text-sm rounded hover:bg-secondary text-left"
                  >
                    Unmark as downloaded
                  </button>
                )}
              />
            ) : isDownloaded ? (
              <span className="px-2 py-1.5 text-sm text-muted-foreground">
                Already downloaded by us
              </span>
            ) : (
              <PopoverClose
                render={(props) => (
                  <button
                    {...props}
                    onClick={handleMark}
                    disabled={isLoading}
                    className="flex items-center gap-2 px-2 py-1.5 text-sm rounded hover:bg-secondary text-left"
                  >
                    Mark as already downloaded
                  </button>
                )}
              />
            )}
          </PopoverContent>
        </Popover>
      </div>
    </div>
  );
}

export function EpisodeListSkeleton() {
  return (
    <div className="flex gap-4 p-3 rounded-xl border bg-card animate-pulse">
      <div className="shrink-0 w-40 aspect-video rounded-lg bg-muted" />
      <div className="flex-1 space-y-2 py-1">
        <div className="h-3 w-12 bg-muted rounded" />
        <div className="h-4 w-48 bg-muted rounded" />
        <div className="h-3 w-full bg-muted rounded" />
      </div>
    </div>
  );
}
