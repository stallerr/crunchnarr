'use client';

import { useState } from 'react';
import { ClapperboardIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { TrackingDialog } from './tracking-dialog';
import {
  useTrackedSeries,
  useAddTracked,
  useUpdateTracked,
  useDeleteTracked,
} from '@/hooks/use-tracking';
import type { TrackingMode } from '@/lib/api/calls/tracking';
import { cn } from '@/lib/utils';

type TrackButtonProps = {
  seriesId: string;
  variant?: 'default' | 'outline' | 'ghost';
  size?: 'default' | 'sm' | 'icon-sm';
};

export function TrackButton({
  seriesId,
  variant = 'outline',
  size = 'sm',
}: TrackButtonProps) {
  const { bySeriesId, refetch } = useTrackedSeries();
  const entry = bySeriesId.get(seriesId) ?? null;
  const [dialogOpen, setDialogOpen] = useState(false);

  const { execute: add, isLoading: adding } = useAddTracked();
  const { execute: update, isLoading: updating } = useUpdateTracked();
  const { execute: remove, isLoading: removing } = useDeleteTracked();

  const isTracked = entry !== null;
  const isSaving = adding || updating;

  const handleConfirm = async (mode: TrackingMode, enabled: boolean) => {
    if (entry) {
      const { error } = await update(entry.id, { download_mode: mode, enabled });
      if (!error) {
        setDialogOpen(false);
        refetch();
      }
    } else {
      const { error } = await add(seriesId, mode);
      if (!error) {
        setDialogOpen(false);
        refetch();
      }
    }
  };

  const handleUntrack = async () => {
    if (!entry) return;
    const { error } = await remove(entry.id);
    if (!error) {
      setDialogOpen(false);
      refetch();
    }
  };

  return (
    <>
      <Button
        variant={variant}
        size={size}
        onClick={() => setDialogOpen(true)}
        aria-pressed={isTracked}
      >
        <ClapperboardIcon className={cn(isTracked && 'fill-current')} />
        {isTracked ? (entry.enabled ? 'Tracked' : 'Tracked (paused)') : 'Track'}
      </Button>
      <TrackingDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        entry={entry}
        isSaving={isSaving}
        onConfirm={handleConfirm}
        onUntrack={isTracked ? handleUntrack : undefined}
        isUntracking={removing}
      />
    </>
  );
}
