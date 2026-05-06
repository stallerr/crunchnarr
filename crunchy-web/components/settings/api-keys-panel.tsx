'use client';

import { useState } from 'react';
import { CopyIcon, KeyIcon, PlusIcon, ShieldAlertIcon, Trash2Icon } from 'lucide-react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogPortal,
  DialogBackdrop,
  DialogPopup,
  DialogTitle,
  DialogDescription,
} from '@/components/ui/dialog';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { toastManager } from '@/components/ui/toast';
import {
  useApiKeys,
  useCreateApiKey,
  useRevokeApiKey,
} from '@/hooks/use-api-keys';
import type { ApiKeyItem, CreateApiKeyResponse } from '@/lib/api/calls/api-keys';

function formatDate(value: string | null): string {
  if (!value) return 'Never';
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return value;
  return d.toLocaleString();
}

export function ApiKeysPanel() {
  const { data: keys, isLoading, error, refetch } = useApiKeys();

  const [createOpen, setCreateOpen] = useState(false);
  const [name, setName] = useState('');
  const { execute: createExecute, isLoading: creating } = useCreateApiKey();

  const [revealed, setRevealed] = useState<CreateApiKeyResponse | null>(null);

  const [revokeTarget, setRevokeTarget] = useState<ApiKeyItem | null>(null);
  const { execute: revokeExecute, isLoading: revoking } = useRevokeApiKey();

  const handleCreate = async () => {
    const trimmed = name.trim();
    if (!trimmed) return;
    const { data, error: err } = await createExecute(trimmed);
    if (err || !data) {
      toastManager.add({
        title: 'Failed to create API key',
        description: err ?? 'Unknown error',
        type: 'error',
        timeout: 5000,
      });
      return;
    }
    setCreateOpen(false);
    setName('');
    setRevealed(data);
    refetch();
  };

  const handleRevoke = async () => {
    if (!revokeTarget) return;
    const { error: err } = await revokeExecute(revokeTarget.id);
    setRevokeTarget(null);
    if (!err) refetch();
  };

  const handleCopy = async (value: string) => {
    try {
      await navigator.clipboard.writeText(value);
      toastManager.add({
        title: 'Copied to clipboard',
        type: 'success',
        timeout: 2000,
      });
    } catch {
      toastManager.add({
        title: 'Copy failed',
        description: 'Select the key and copy it manually.',
        type: 'error',
        timeout: 4000,
      });
    }
  };

  return (
    <>
      <Card>
        <CardHeader>
          <CardTitle>API Keys</CardTitle>
          <CardDescription>
            Use API keys to authenticate non-interactive clients with{' '}
            <code className="text-xs">X-Api-Key: &lt;key&gt;</code>. Keys carry the same permissions
            as your account.
          </CardDescription>
        </CardHeader>
        <CardPanel>
          <div className="flex justify-end mb-4">
            <Button onClick={() => setCreateOpen(true)}>
              <PlusIcon /> Create API Key
            </Button>
          </div>

          {isLoading ? (
            <div className="h-32 rounded-lg border bg-card animate-pulse" />
          ) : error ? (
            <div className="py-8 text-center text-sm text-muted-foreground">{error}</div>
          ) : !keys || keys.length === 0 ? (
            <div className="flex flex-col items-center gap-2 py-12 text-muted-foreground">
              <KeyIcon className="size-10 opacity-50" />
              <p className="text-sm">No API keys yet.</p>
            </div>
          ) : (
            <div className="overflow-x-auto rounded-lg border">
              <table className="w-full text-sm">
                <thead className="bg-secondary/40 text-left text-xs uppercase text-muted-foreground">
                  <tr>
                    <th className="px-4 py-2 font-medium">Name</th>
                    <th className="px-4 py-2 font-medium">Prefix</th>
                    <th className="px-4 py-2 font-medium">Created</th>
                    <th className="px-4 py-2 font-medium">Last Used</th>
                    <th className="px-4 py-2 font-medium w-px"></th>
                  </tr>
                </thead>
                <tbody>
                  {keys.map((k) => (
                    <tr key={k.id} className="border-t">
                      <td className="px-4 py-2.5 font-medium">{k.name}</td>
                      <td className="px-4 py-2.5">
                        <code className="rounded bg-secondary px-1.5 py-0.5 text-xs">
                          crl_{k.key_prefix}…
                        </code>
                      </td>
                      <td className="px-4 py-2.5 text-muted-foreground">
                        {formatDate(k.created_at)}
                      </td>
                      <td className="px-4 py-2.5 text-muted-foreground">
                        {formatDate(k.last_used_at)}
                      </td>
                      <td className="px-4 py-2.5">
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          onClick={() => setRevokeTarget(k)}
                          aria-label={`Revoke ${k.name}`}
                        >
                          <Trash2Icon className="text-destructive" />
                        </Button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </CardPanel>
      </Card>

      {/* Create dialog */}
      <Dialog open={createOpen} onOpenChange={setCreateOpen}>
        <DialogPortal>
          <DialogBackdrop />
          <DialogPopup className="max-w-md">
            <DialogTitle>Create API Key</DialogTitle>
            <DialogDescription className="mt-2">
              Pick a name to identify where this key will be used.
            </DialogDescription>
            <div className="mt-4">
              <Field>
                <FieldLabel>Name</FieldLabel>
                <Input
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g. Home Assistant"
                  maxLength={100}
                  autoFocus
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && name.trim() && !creating) {
                      e.preventDefault();
                      handleCreate();
                    }
                  }}
                />
              </Field>
            </div>
            <div className="flex items-center justify-end gap-3 mt-6">
              <Button
                variant="ghost"
                onClick={() => {
                  setCreateOpen(false);
                  setName('');
                }}
                disabled={creating}
              >
                Cancel
              </Button>
              <Button onClick={handleCreate} disabled={!name.trim() || creating}>
                {creating ? 'Creating…' : 'Create'}
              </Button>
            </div>
          </DialogPopup>
        </DialogPortal>
      </Dialog>

      {/* One-time reveal dialog */}
      <Dialog open={revealed !== null} onOpenChange={(open) => !open && setRevealed(null)}>
        <DialogPortal>
          <DialogBackdrop />
          <DialogPopup className="max-w-lg">
            <DialogTitle>API Key Created</DialogTitle>
            <DialogDescription className="mt-2">
              Store this key now — you won&apos;t be able to see it again.
            </DialogDescription>
            {revealed && (
              <>
                <div className="mt-4 flex items-center gap-2 rounded-lg border bg-amber-500/10 p-3 text-amber-700 dark:text-amber-400">
                  <ShieldAlertIcon className="size-4 shrink-0" />
                  <p className="text-xs">
                    The full key is shown once. If you lose it, revoke it and create a new one.
                  </p>
                </div>
                <div className="mt-4 flex items-center gap-2 rounded-lg border bg-secondary/40 p-3">
                  <code className="flex-1 break-all font-mono text-xs">{revealed.key}</code>
                  <Button
                    variant="outline"
                    size="icon-sm"
                    onClick={() => handleCopy(revealed.key)}
                    aria-label="Copy key"
                  >
                    <CopyIcon />
                  </Button>
                </div>
              </>
            )}
            <div className="flex items-center justify-end gap-3 mt-6">
              <Button onClick={() => setRevealed(null)}>Done</Button>
            </div>
          </DialogPopup>
        </DialogPortal>
      </Dialog>

      {/* Revoke confirmation */}
      <ConfirmDialog
        open={revokeTarget !== null}
        onOpenChange={(open) => !open && setRevokeTarget(null)}
        title="Revoke API key?"
        description={
          revokeTarget
            ? `“${revokeTarget.name}” will stop working immediately. This cannot be undone.`
            : ''
        }
        confirmLabel={revoking ? 'Revoking…' : 'Revoke'}
        variant="destructive"
        onConfirm={handleRevoke}
      />
    </>
  );
}
