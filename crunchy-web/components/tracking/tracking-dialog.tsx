'use client';

import { useEffect, useState } from 'react';
import { Trash2Icon } from 'lucide-react';
import {
  Dialog,
  DialogPortal,
  DialogBackdrop,
  DialogPopup,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel } from '@/components/ui/field';
import { RadioGroup, Radio } from '@/components/ui/radio-group';
import { Checkbox } from '@/components/ui/checkbox';
import type { TrackingMode, TrackedSeriesItem } from '@/lib/api/calls/tracking';

type TrackingDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Pre-fill from an existing entry (edit mode) or null (create mode). */
  entry: TrackedSeriesItem | null;
  /** Saving state — disables form controls. */
  isSaving?: boolean;
  /** Called with the chosen mode + enabled flag. Edit mode passes both. */
  onConfirm: (mode: TrackingMode, enabled: boolean) => void;
  /** Edit mode only — called when the user confirms Untrack. */
  onUntrack?: () => void;
  isUntracking?: boolean;
};

export function TrackingDialog({
  open,
  onOpenChange,
  entry,
  isSaving = false,
  onConfirm,
  onUntrack,
  isUntracking = false,
}: TrackingDialogProps) {
  const isEdit = entry !== null;
  const [mode, setMode] = useState<TrackingMode>(entry?.download_mode ?? 'new_only');
  const [paused, setPaused] = useState<boolean>(entry ? !entry.enabled : false);
  const [confirmRemove, setConfirmRemove] = useState(false);

  // Re-sync local state whenever the dialog opens or the entry changes.
  useEffect(() => {
    if (open) {
      setMode(entry?.download_mode ?? 'new_only');
      setPaused(entry ? !entry.enabled : false);
    }
  }, [open, entry]);

  const handleConfirm = () => {
    onConfirm(mode, !paused);
  };

  const handleUntrack = () => {
    setConfirmRemove(false);
    onUntrack?.();
  };

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogPortal>
          <DialogBackdrop />
          <DialogPopup className="max-w-md">
            <DialogTitle>{isEdit ? 'Edit watchlist entry' : 'Track this series'}</DialogTitle>
            <DialogDescription className="mt-1">
              {isEdit
                ? 'Adjust how this series is auto-downloaded, pause it, or remove it from your watchlist.'
                : 'Choose how new episodes should be downloaded automatically.'}
            </DialogDescription>

            <div className="mt-5 space-y-5">
              <Field>
                <FieldLabel>Download mode</FieldLabel>
                <RadioGroup
                  value={mode}
                  onValueChange={(v) => setMode(v as TrackingMode)}
                  className="flex flex-col gap-3"
                >
                  <label className="flex items-start gap-3 rounded-xl border border-border bg-secondary/40 px-4 py-3 cursor-pointer transition-colors hover:bg-secondary/70 has-data-checked:border-primary has-data-checked:bg-primary/8">
                    <Radio value="new_only" className="mt-0.5" />
                    <div>
                      <span className="text-sm font-semibold">New episodes only</span>
                      <p className="text-xs text-muted-foreground mt-0.5">
                        Skip the existing back catalog. Only download episodes released from now on.
                      </p>
                    </div>
                  </label>
                  <label className="flex items-start gap-3 rounded-xl border border-border bg-secondary/40 px-4 py-3 cursor-pointer transition-colors hover:bg-secondary/70 has-data-checked:border-primary has-data-checked:bg-primary/8">
                    <Radio value="all" className="mt-0.5" />
                    <div>
                      <span className="text-sm font-semibold">Download everything</span>
                      <p className="text-xs text-muted-foreground mt-0.5">
                        Backfill the whole series, then keep up with new episodes.
                      </p>
                    </div>
                  </label>
                </RadioGroup>
              </Field>

              {isEdit && (
                <label className="flex items-start gap-3 rounded-xl border border-border bg-secondary/40 px-4 py-3 cursor-pointer transition-colors hover:bg-secondary/70">
                  <Checkbox
                    checked={paused}
                    onCheckedChange={(c) => setPaused(c === true)}
                    className="mt-0.5"
                  />
                  <div>
                    <span className="text-sm font-semibold">Pause auto-download</span>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      Stop the polling worker from acting on this series. Existing downloads stay.
                    </p>
                  </div>
                </label>
              )}
            </div>

            <div className="flex items-center justify-between gap-3 mt-6">
              {isEdit && onUntrack ? (
                <Button
                  variant="destructive-outline"
                  onClick={() => setConfirmRemove(true)}
                  disabled={isSaving || isUntracking}
                >
                  <Trash2Icon />
                  Untrack
                </Button>
              ) : (
                <span />
              )}
              <div className="flex items-center gap-3">
                <Button
                  variant="ghost"
                  onClick={() => onOpenChange(false)}
                  disabled={isSaving}
                >
                  Cancel
                </Button>
                <Button onClick={handleConfirm} disabled={isSaving}>
                  {isSaving ? 'Saving…' : isEdit ? 'Save' : 'Track'}
                </Button>
              </div>
            </div>
          </DialogPopup>
        </DialogPortal>
      </Dialog>

      <ConfirmDialog
        open={confirmRemove}
        onOpenChange={setConfirmRemove}
        title="Remove from watchlist?"
        description="Auto-download stops immediately. Existing downloads stay."
        confirmLabel={isUntracking ? 'Removing…' : 'Untrack'}
        variant="destructive"
        onConfirm={handleUntrack}
      />
    </>
  );
}
