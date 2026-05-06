'use client';

import {
  Dialog,
  DialogPortal,
  DialogBackdrop,
  DialogPopup,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { formatBytes } from '@/lib/format';

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  entryCount: number;
  totalSizeBytes: number;
  isLoading: boolean;
  onConfirm: () => void;
};

export function CleanCacheDialog({
  open,
  onOpenChange,
  entryCount,
  totalSizeBytes,
  isLoading,
  onConfirm,
}: Props) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPortal>
        <DialogBackdrop />
        <DialogPopup className="max-w-sm">
          <DialogTitle>Clean All Cache</DialogTitle>
          <DialogDescription className="mt-2">
            This will permanently delete{' '}
            <strong>{entryCount} cache {entryCount === 1 ? 'entry' : 'entries'}</strong> totalling{' '}
            <strong>{formatBytes(totalSizeBytes)}</strong>. In-progress downloads may be
            interrupted.
          </DialogDescription>
          <div className="flex items-center justify-end gap-3 mt-6">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={onConfirm} disabled={isLoading}>
              {isLoading ? 'Cleaning...' : 'Clean Cache'}
            </Button>
          </div>
        </DialogPopup>
      </DialogPortal>
    </Dialog>
  );
}
