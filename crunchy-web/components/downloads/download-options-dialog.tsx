'use client';

import { DownloadIcon } from 'lucide-react';
import {
  Dialog,
  DialogPortal,
  DialogBackdrop,
  DialogPopup,
  DialogTitle,
  DialogDescription,
  DialogCloseX,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';

type DownloadOptionsDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  episodeTitle: string;
  onDownload: () => void;
  isLoading?: boolean;
};

export function DownloadOptionsDialog({
  open,
  onOpenChange,
  episodeTitle,
  onDownload,
  isLoading,
}: DownloadOptionsDialogProps) {
  const handleDownload = () => {
    onDownload();
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPortal>
        <DialogBackdrop />
        <DialogPopup>
          <DialogCloseX />
          <DialogTitle>Confirm Download</DialogTitle>
          <DialogDescription className="mt-1">
            Download{' '}
            <span className="font-medium text-foreground">{episodeTitle}</span>
            {' '}using your saved settings?
          </DialogDescription>

          <div className="flex items-center justify-end gap-3 mt-6">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button onClick={handleDownload} disabled={isLoading}>
              <DownloadIcon />
              Download
            </Button>
          </div>
        </DialogPopup>
      </DialogPortal>
    </Dialog>
  );
}
