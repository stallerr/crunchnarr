'use client';

import { useState } from 'react';
import {
  PauseIcon,
  PlayIcon,
  XIcon,
  RotateCcwIcon,
  Trash2Icon,
  DownloadIcon,
  LoaderIcon,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { useDownloadActions } from '@/hooks/use-downloads';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { useConfirmCancel } from '@/components/providers/confirm-cancel-provider';
import { toastManager } from '@/components/ui/toast';
import { BASE_URL } from '@/lib/api/client';
import type { DownloadRow } from '@/types/downloads';

type DownloadActionsProps = {
  download: DownloadRow;
  onActionComplete?: () => void;
};

export function DownloadActions({
  download,
  onActionComplete,
}: DownloadActionsProps) {
  const { pause, resume, cancel, loadingId } = useDownloadActions();
  const { getToken } = useAuthToken();
  const { skipConfirm } = useConfirmCancel();
  const [confirmCancelOpen, setConfirmCancelOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const isActing = loadingId === download.id;

  const requestCancel = () => {
    if (skipConfirm) {
      handleCancel();
    } else {
      setConfirmCancelOpen(true);
    }
  };

  const handleSaveFile = async () => {
    const token = await getToken();
    if (!token) return;
    setIsSaving(true);
    try {
      const res = await fetch(`${BASE_URL}/downloads/${download.id}/file`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (!res.ok) {
        const body = await res.json().catch(() => null);
        toastManager.add({
          type: 'error',
          title: 'File unavailable',
          description: body?.message ?? 'File no longer exists at the download location',
        });
        return;
      }
      const blob = await res.blob();
      const disposition = res.headers.get('content-disposition');
      const match = disposition?.match(/filename="(.+)"/);
      const filename = match?.[1] ?? 'download';
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      a.remove();
      URL.revokeObjectURL(url);
    } catch {
      toastManager.add({ type: 'error', title: 'Download failed' });
    } finally {
      setIsSaving(false);
    }
  };

  const handlePause = async () => {
    const { error } = await pause(download.id);
    if (error) {
      toastManager.add({ type: 'error', title: 'Failed to pause', description: error });
    } else {
      toastManager.add({ type: 'success', title: 'Download paused' });
      onActionComplete?.();
    }
  };

  const handleResume = async () => {
    const { error } = await resume(download.id);
    if (error) {
      toastManager.add({ type: 'error', title: 'Failed to resume', description: error });
    } else {
      toastManager.add({ type: 'success', title: 'Download resumed' });
      onActionComplete?.();
    }
  };

  const handleCancel = async () => {
    const { error } = await cancel(download.id);
    if (error) {
      toastManager.add({ type: 'error', title: 'Failed to cancel', description: error });
    } else {
      toastManager.add({ type: 'success', title: 'Download cancelled' });
      onActionComplete?.();
    }
    setConfirmCancelOpen(false);
  };

  const handleRetry = async () => {
    const { error } = await resume(download.id);
    if (error) {
      toastManager.add({ type: 'error', title: 'Failed to retry', description: error });
    } else {
      toastManager.add({ type: 'success', title: 'Retrying download' });
      onActionComplete?.();
    }
  };

  return (
    <>
      {download.status === 'active' && (
        <>
          <Button
            variant="ghost"
            size="icon-xs"
            onClick={handlePause}
            disabled={isActing}
            title="Pause"
          >
            <PauseIcon />
          </Button>
          <Button
            variant="ghost"
            size="icon-xs"
            onClick={requestCancel}
            disabled={isActing}
            title="Cancel"
          >
            <XIcon />
          </Button>
        </>
      )}

      {download.status === 'paused' && (
        <>
          <Button
            variant="ghost"
            size="icon-xs"
            onClick={handleResume}
            disabled={isActing}
            title="Resume"
          >
            <PlayIcon />
          </Button>
          <Button
            variant="ghost"
            size="icon-xs"
            onClick={requestCancel}
            disabled={isActing}
            title="Cancel"
          >
            <XIcon />
          </Button>
        </>
      )}

      {download.status === 'failed' && (
        <>
          <Button
            variant="ghost"
            size="icon-xs"
            onClick={handleRetry}
            disabled={isActing}
            title="Retry"
          >
            <RotateCcwIcon />
          </Button>
          <Button
            variant="ghost"
            size="icon-xs"
            onClick={requestCancel}
            disabled={isActing}
            title="Remove"
          >
            <Trash2Icon />
          </Button>
        </>
      )}

      {download.status === 'completed' && download.output_path && (
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={handleSaveFile}
          disabled={isSaving}
          title={isSaving ? 'Preparing file…' : 'Save file'}
        >
          {isSaving ? <LoaderIcon className="animate-spin" /> : <DownloadIcon />}
        </Button>
      )}

      {download.status === 'pending' && (
        <Button
          variant="ghost"
          size="icon-xs"
          onClick={requestCancel}
          disabled={isActing}
          title="Cancel"
        >
          <XIcon />
        </Button>
      )}

      <ConfirmDialog
        open={confirmCancelOpen}
        onOpenChange={setConfirmCancelOpen}
        title="Cancel Download"
        description={`Are you sure you want to cancel the download for "${download.episode_title}"?`}
        confirmLabel="Cancel Download"
        variant="destructive"
        onConfirm={handleCancel}
      />
    </>
  );
}
