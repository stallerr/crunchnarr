'use client';

import { useState } from 'react';
import { DownloadIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { DownloadOptionsDialog } from './download-options-dialog';
import { useStartDownload } from '@/hooks/use-downloads';
import { toastManager } from '@/components/ui/toast';
import type { CREpisode } from '@/types/crunchyroll';
import { DEFAULT_DOWNLOAD_OPTIONS } from '@/types/download-options';

type DownloadButtonProps = {
  episode: CREpisode;
  variant?: 'default' | 'ghost' | 'outline';
  size?: 'default' | 'sm' | 'icon-sm';
  showLabel?: boolean;
};

export function DownloadButton({
  episode,
  variant = 'default',
  size = 'default',
  showLabel = true,
}: DownloadButtonProps) {
  const [dialogOpen, setDialogOpen] = useState(false);
  const { execute, isLoading } = useStartDownload();

  const handleDownload = async () => {
    const { data, error } = await execute(episode.id, DEFAULT_DOWNLOAD_OPTIONS);
    if (error) {
      toastManager.add({
        type: 'error',
        title: 'Download failed',
        description: error,
      });
    } else if (data) {
      toastManager.add({
        type: 'success',
        title: 'Download started',
        description: `${data.length} item${data.length !== 1 ? 's' : ''} started`,
      });
    }
  };

  return (
    <>
      <Button
        variant={variant}
        size={size}
        onClick={() => setDialogOpen(true)}
        disabled={isLoading}
      >
        <DownloadIcon />
        {showLabel && 'Download'}
      </Button>
      <DownloadOptionsDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        episodeTitle={episode.title}
        onDownload={handleDownload}
        isLoading={isLoading}
      />
    </>
  );
}
