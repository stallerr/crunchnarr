'use client';

import { useState } from 'react';
import {
  Dialog,
  DialogPortal,
  DialogBackdrop,
  DialogPopup,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { post, unwrap } from '@/lib/api/client';
import { toastManager } from '@/components/ui/toast';

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onUnlinked: () => void;
};

export function UnlinkConfirmationDialog({ open, onOpenChange, onUnlinked }: Props) {
  const { getToken } = useAuthToken();
  const [isLoading, setIsLoading] = useState(false);

  const handleUnlink = async () => {
    setIsLoading(true);
    try {
      const token = await getToken();
      if (!token) return;
      const { error } = await unwrap(post(token, '/crunchyroll/logout'));
      if (!error) {
        toastManager.add({
          title: 'Crunchyroll account unlinked',
          type: 'success',
          timeout: 4000,
        });
        onOpenChange(false);
        onUnlinked();
      } else {
        toastManager.add({
          title: 'Failed to unlink account',
          description: error,
          type: 'error',
          timeout: 5000,
        });
      }
    } catch {
      toastManager.add({
        title: 'An unexpected error occurred',
        type: 'error',
        timeout: 5000,
      });
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPortal>
        <DialogBackdrop />
        <DialogPopup className="max-w-sm">
          <DialogTitle>Unlink Crunchyroll Account</DialogTitle>
          <DialogDescription className="mt-2">
            This will remove your stored Crunchyroll credentials. You will no longer be able to
            search or download content until you re-link your account.
          </DialogDescription>
          <div className="flex items-center justify-end gap-3 mt-6">
            <Button variant="ghost" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button variant="destructive" onClick={handleUnlink} disabled={isLoading}>
              {isLoading ? 'Unlinking...' : 'Unlink Account'}
            </Button>
          </div>
        </DialogPopup>
      </DialogPortal>
    </Dialog>
  );
}
