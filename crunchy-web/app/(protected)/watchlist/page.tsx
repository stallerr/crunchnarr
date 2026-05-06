'use client';

import { useEffect, useMemo, useState } from 'react';
import Link from 'next/link';
import {
  ClapperboardIcon,
  PauseIcon,
  PlayIcon,
  PlusIcon,
  RefreshCwIcon,
  SearchIcon,
  Trash2Icon,
  XIcon,
} from 'lucide-react';
import {
  PagePanel,
  PageHeader,
  PageTitle,
  PageDescription,
} from '@/components/layout/page';
import {
  Dialog,
  DialogPortal,
  DialogBackdrop,
  DialogPopup,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { CRImage } from '@/components/ui/cr-image';
import { TrackingDialog } from '@/components/tracking/tracking-dialog';
import {
  useTrackedSeries,
  useAddTracked,
  useUpdateTracked,
  useDeleteTracked,
  useCheckTracked,
} from '@/hooks/use-tracking';
import { useSearch } from '@/hooks/use-search';
import type { TrackedSeriesItem, TrackingMode } from '@/lib/api/calls/tracking';
import type { CRSearchItem } from '@/types/crunchyroll';
import { cn } from '@/lib/utils';

function relativeTime(iso: string | null): string {
  if (!iso) return 'Never';
  const t = new Date(iso).getTime();
  if (Number.isNaN(t)) return iso;
  const diff = Date.now() - t;
  const m = Math.round(diff / 60_000);
  if (m < 1) return 'Just now';
  if (m < 60) return `${m}m ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h}h ago`;
  const d = Math.round(h / 24);
  return `${d}d ago`;
}

export default function WatchlistPage() {
  const { data, isLoading, error, refetch } = useTrackedSeries();
  const [editingEntry, setEditingEntry] = useState<TrackedSeriesItem | null>(null);
  const [showAdd, setShowAdd] = useState(false);

  const { execute: update, isLoading: updating } = useUpdateTracked();
  const { execute: remove, isLoading: removing } = useDeleteTracked();
  const { execute: checkNow, isLoading: checking } = useCheckTracked();
  const [checkingId, setCheckingId] = useState<string | null>(null);

  const handleEditConfirm = async (mode: TrackingMode, enabled: boolean) => {
    if (!editingEntry) return;
    const { error } = await update(editingEntry.id, { download_mode: mode, enabled });
    if (!error) {
      setEditingEntry(null);
      refetch();
    }
  };

  const handleEditUntrack = async () => {
    if (!editingEntry) return;
    const { error } = await remove(editingEntry.id);
    if (!error) {
      setEditingEntry(null);
      refetch();
    }
  };

  const handleCheckNow = async (id: string) => {
    setCheckingId(id);
    await checkNow(id);
    setCheckingId(null);
    refetch();
  };

  const handleTogglePause = async (entry: TrackedSeriesItem) => {
    const { error } = await update(entry.id, { enabled: !entry.enabled });
    if (!error) refetch();
  };

  return (
    <PagePanel>
      <PageHeader>
        <div className="flex items-start justify-between gap-4">
          <div>
            <div className="flex items-center gap-2">
              <ClapperboardIcon className="size-6 text-primary" />
              <PageTitle>Watchlist</PageTitle>
            </div>
            <PageDescription>
              Series here are polled hourly. New episodes are downloaded automatically and
              completed episodes are upgraded when new dub/sub tracks become available.
            </PageDescription>
          </div>
          <Button onClick={() => setShowAdd(true)}>
            <PlusIcon /> Add Show
          </Button>
        </div>
      </PageHeader>

      {isLoading ? (
        <div className="space-y-2">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="h-20 rounded-xl border bg-card animate-pulse" />
          ))}
        </div>
      ) : error ? (
        <div className="flex flex-col items-center py-16 text-muted-foreground">
          <p className="text-sm">{error}</p>
        </div>
      ) : !data || data.length === 0 ? (
        <div className="flex flex-col items-center gap-4 py-20 text-muted-foreground">
          <ClapperboardIcon className="size-12 opacity-50" />
          <p className="text-sm">Nothing tracked yet.</p>
          <Button onClick={() => setShowAdd(true)} variant="outline">
            <PlusIcon /> Add a show
          </Button>
        </div>
      ) : (
        <div className="space-y-2">
          {data.map((entry) => (
            <div
              key={entry.id}
              className={cn(
                'flex items-center gap-4 rounded-xl border bg-card p-3',
                !entry.enabled && 'opacity-60'
              )}
            >
              <Link
                href={`/series/${entry.series_id}`}
                className="shrink-0 w-12 h-16 rounded-md overflow-hidden bg-muted"
              >
                {entry.series_thumbnail ? (
                  <CRImage
                    images={{ poster_tall: [[{ source: entry.series_thumbnail, width: 100, height: 150 }]] }}
                    type="tall"
                    preferredWidth={100}
                    alt={entry.series_title}
                    className="w-full h-full object-cover"
                  />
                ) : null}
              </Link>
              <div className="flex-1 min-w-0">
                <Link
                  href={`/series/${entry.series_id}`}
                  className="font-medium text-sm line-clamp-1 hover:underline"
                >
                  {entry.series_title}
                </Link>
                <div className="flex items-center gap-2 mt-0.5 text-xs text-muted-foreground">
                  <span>
                    {entry.download_mode === 'new_only' ? 'New episodes only' : 'All episodes'}
                  </span>
                  <span>·</span>
                  <span>Last checked {relativeTime(entry.last_checked_at)}</span>
                  {!entry.enabled && (
                    <>
                      <span>·</span>
                      <span className="text-amber-500">Paused</span>
                    </>
                  )}
                </div>
              </div>
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => handleTogglePause(entry)}
                  disabled={updating}
                  aria-label={entry.enabled ? 'Pause' : 'Resume'}
                  title={entry.enabled ? 'Pause auto-download' : 'Resume auto-download'}
                >
                  {entry.enabled ? <PauseIcon /> : <PlayIcon />}
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => handleCheckNow(entry.id)}
                  disabled={checking && checkingId === entry.id}
                  aria-label="Check now"
                  title="Check for new episodes now"
                >
                  <RefreshCwIcon className={cn(checking && checkingId === entry.id && 'animate-spin')} />
                </Button>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  onClick={() => setEditingEntry(entry)}
                  aria-label="Edit"
                  title="Edit watchlist entry"
                >
                  <ClapperboardIcon />
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      <TrackingDialog
        open={editingEntry !== null}
        onOpenChange={(open) => !open && setEditingEntry(null)}
        entry={editingEntry}
        isSaving={updating}
        onConfirm={handleEditConfirm}
        onUntrack={handleEditUntrack}
        isUntracking={removing}
      />

      <AddShowDialog
        open={showAdd}
        onOpenChange={setShowAdd}
        onAdded={() => {
          setShowAdd(false);
          refetch();
        }}
      />
    </PagePanel>
  );
}

type AddShowDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdded: () => void;
};

function AddShowDialog({ open, onOpenChange, onAdded }: AddShowDialogProps) {
  const [query, setQuery] = useState('');
  const { results, isLoading, search, clear } = useSearch();
  const [picked, setPicked] = useState<CRSearchItem | null>(null);
  const { bySeriesId } = useTrackedSeries();
  const { execute: add, isLoading: adding } = useAddTracked();

  const seriesResults = useMemo(() => {
    return results
      .flatMap((g) => g.items)
      .filter((i) => i.type === 'series');
  }, [results]);

  useEffect(() => {
    if (open) {
      setQuery('');
      setPicked(null);
      clear();
    }
  }, [open, clear]);

  useEffect(() => {
    if (!open) return;
    search(query, 10);
  }, [query, open, search]);

  const handleConfirm = async (mode: TrackingMode) => {
    if (!picked) return;
    const { error } = await add(picked.id, mode);
    if (!error) {
      onAdded();
    }
  };

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogPortal>
          <DialogBackdrop />
          <DialogPopup className="max-w-xl">
            <DialogTitle>Add show to watchlist</DialogTitle>
            <DialogDescription className="mt-1">
              Search Crunchyroll for a series, then choose how it should be auto-downloaded.
            </DialogDescription>

            <div className="mt-4 relative">
              <Input
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Type a show name…"
                autoFocus
              >
                {query && (
                  <button
                    type="button"
                    onClick={() => setQuery('')}
                    className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-secondary"
                    aria-label="Clear"
                  >
                    <XIcon className="size-4" />
                  </button>
                )}
              </Input>
            </div>

            <div className="mt-4 max-h-80 overflow-y-auto space-y-1">
              {!query.trim() ? (
                <div className="flex flex-col items-center py-10 text-muted-foreground">
                  <SearchIcon className="size-8 opacity-50" />
                  <p className="text-xs mt-2">Start typing to search</p>
                </div>
              ) : isLoading ? (
                <div className="py-6 text-center text-sm text-muted-foreground">Searching…</div>
              ) : seriesResults.length === 0 ? (
                <div className="py-6 text-center text-sm text-muted-foreground">No series found.</div>
              ) : (
                seriesResults.map((item) => {
                  const alreadyTracked = bySeriesId.has(item.id);
                  return (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => !alreadyTracked && setPicked(item)}
                      disabled={alreadyTracked}
                      className={cn(
                        'w-full flex items-center gap-3 p-2 rounded-lg transition-colors text-left',
                        alreadyTracked
                          ? 'opacity-50 cursor-not-allowed'
                          : 'hover:bg-secondary/60 cursor-pointer'
                      )}
                    >
                      <div className="shrink-0 w-10 h-14 rounded-md overflow-hidden bg-muted">
                        <CRImage
                          images={item.images}
                          type="tall"
                          preferredWidth={80}
                          alt={item.title}
                          className="w-full h-full object-cover"
                        />
                      </div>
                      <div className="flex-1 min-w-0">
                        <p className="font-medium text-sm line-clamp-1">{item.title}</p>
                        <p className="text-xs text-muted-foreground line-clamp-2">{item.description}</p>
                      </div>
                      {alreadyTracked && (
                        <span className="shrink-0 text-xs text-muted-foreground">Already tracked</span>
                      )}
                    </button>
                  );
                })
              )}
            </div>

            <div className="flex items-center justify-end gap-3 mt-4">
              <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={adding}>
                Cancel
              </Button>
            </div>
          </DialogPopup>
        </DialogPortal>
      </Dialog>

      <TrackingDialog
        open={picked !== null}
        onOpenChange={(open) => !open && setPicked(null)}
        entry={null}
        isSaving={adding}
        onConfirm={(mode) => handleConfirm(mode)}
      />
    </>
  );
}
