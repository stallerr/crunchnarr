'use client';

import { useRef, useState } from 'react';
import { FileKeyIcon, ShieldCheckIcon, UploadCloudIcon, XIcon } from 'lucide-react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel, FieldDescription } from '@/components/ui/field';
import {
  NumberField,
  NumberFieldGroup,
  NumberFieldInput,
  NumberFieldIncrement,
  NumberFieldDecrement,
} from '@/components/ui/number-field';
import { useUpdateConfig } from '@/hooks/use-config';
import type { AppConfig } from '@/lib/api/calls/config';
import { cn } from '@/lib/utils';

type Props = {
  config: AppConfig;
  onSaved: () => void;
};

const SECRET_PLACEHOLDER = '********';

/** Read a File as base64 (no data: prefix). */
async function fileToBase64(file: File): Promise<string> {
  const buf = await file.arrayBuffer();
  const bytes = new Uint8Array(buf);
  let binary = '';
  for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
  return btoa(binary);
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

type FileStatus = 'unset' | 'stored' | 'legacy-path';

function describeStored(value: string | undefined): { kind: FileStatus; label: string } {
  if (!value) return { kind: 'unset', label: 'No file uploaded yet' };
  if (value === SECRET_PLACEHOLDER) return { kind: 'stored', label: 'Encrypted blob stored on the server' };
  return { kind: 'legacy-path', label: value };
}

type FilePickerFieldProps = {
  label: string;
  description: React.ReactNode;
  accept: string;
  pendingFile: File | null;
  onPick: (file: File | null) => void;
  storedStatus: { kind: FileStatus; label: string };
  inputRef: React.RefObject<HTMLInputElement | null>;
};

function FilePickerField({
  label,
  description,
  accept,
  pendingFile,
  onPick,
  storedStatus,
  inputRef,
}: FilePickerFieldProps) {
  const [dragOver, setDragOver] = useState(false);

  const handleClick = () => inputRef.current?.click();
  const handleClear = (e: React.MouseEvent) => {
    e.stopPropagation();
    onPick(null);
    if (inputRef.current) inputRef.current.value = '';
  };
  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const file = e.dataTransfer.files?.[0];
    if (file) onPick(file);
  };

  const showStatusRow = !pendingFile && storedStatus.kind !== 'unset';

  return (
    <Field>
      <FieldLabel>{label}</FieldLabel>
      <input
        ref={inputRef}
        type="file"
        accept={accept}
        onChange={(e) => onPick((e.target as HTMLInputElement).files?.[0] ?? null)}
        className="sr-only"
      />
      <button
        type="button"
        onClick={handleClick}
        onDragOver={(e) => {
          e.preventDefault();
          setDragOver(true);
        }}
        onDragLeave={() => setDragOver(false)}
        onDrop={handleDrop}
        className={cn(
          'group flex w-full items-center gap-3 rounded-lg border border-dashed border-input bg-background/50 p-3 text-left transition-colors',
          'hover:border-primary/50 hover:bg-secondary/40',
          dragOver && 'border-primary bg-primary/5',
          pendingFile && 'border-primary/50 bg-primary/5'
        )}
      >
        <div
          className={cn(
            'flex size-10 shrink-0 items-center justify-center rounded-md border bg-muted/40',
            pendingFile && 'border-primary/50 bg-primary/10 text-primary'
          )}
        >
          {pendingFile ? (
            <FileKeyIcon className="size-5" />
          ) : (
            <UploadCloudIcon className="size-5 text-muted-foreground" />
          )}
        </div>
        <div className="min-w-0 flex-1">
          {pendingFile ? (
            <>
              <p className="truncate text-sm font-medium">{pendingFile.name}</p>
              <p className="text-xs text-muted-foreground">
                {formatSize(pendingFile.size)} — ready to upload
              </p>
            </>
          ) : (
            <>
              <p className="text-sm font-medium">Choose a file or drop it here</p>
              <p className="text-xs text-muted-foreground">
                Accepted: <code className="text-[11px]">{accept}</code>
              </p>
            </>
          )}
        </div>
        {pendingFile ? (
          <span
            role="button"
            tabIndex={0}
            onClick={handleClear}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                handleClear(e as unknown as React.MouseEvent);
              }
            }}
            className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-secondary hover:text-foreground"
            aria-label="Remove selected file"
          >
            <XIcon className="size-4" />
          </span>
        ) : (
          <span className="shrink-0 rounded-md border bg-background px-2.5 py-1 text-xs font-medium">
            Browse
          </span>
        )}
      </button>
      {showStatusRow && (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          {storedStatus.kind === 'stored' ? (
            <ShieldCheckIcon className="size-3.5 text-emerald-500" />
          ) : null}
          <span className="truncate">{storedStatus.label}</span>
        </div>
      )}
      <FieldDescription>{description}</FieldDescription>
    </Field>
  );
}

export function WidevineSettingsForm({ config, onSaved }: Props) {
  const [clientStored, setClientStored] = useState(describeStored(config.widevine_client));
  const [privateKeyStored, setPrivateKeyStored] = useState(
    describeStored(config.widevine_private_key)
  );
  const [pendingClient, setPendingClient] = useState<File | null>(null);
  const [pendingPrivateKey, setPendingPrivateKey] = useState<File | null>(null);
  const [concurrentKeyAcquisitions, setConcurrentKeyAcquisitions] = useState(
    config.concurrent_key_acquisitions ?? 2
  );
  const clientInputRef = useRef<HTMLInputElement>(null);
  const privateKeyInputRef = useRef<HTMLInputElement>(null);
  const { execute, isLoading } = useUpdateConfig();

  const handleSave = async () => {
    const updates: Partial<AppConfig> = {
      concurrent_key_acquisitions: concurrentKeyAcquisitions,
    };
    updates.widevine_client = pendingClient
      ? await fileToBase64(pendingClient)
      : SECRET_PLACEHOLDER;
    updates.widevine_private_key = pendingPrivateKey
      ? await fileToBase64(pendingPrivateKey)
      : SECRET_PLACEHOLDER;

    const { error } = await execute(updates);
    if (error) return;

    if (pendingClient) {
      setClientStored({ kind: 'stored', label: `Encrypted ${pendingClient.name} stored` });
      setPendingClient(null);
      if (clientInputRef.current) clientInputRef.current.value = '';
    }
    if (pendingPrivateKey) {
      setPrivateKeyStored({
        kind: 'stored',
        label: `Encrypted ${pendingPrivateKey.name} stored`,
      });
      setPendingPrivateKey(null);
      if (privateKeyInputRef.current) privateKeyInputRef.current.value = '';
    }
    onSaved();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Widevine DRM</CardTitle>
        <CardDescription>
          Upload your Widevine <code>client_id.bin</code> and <code>private_key.pem</code>. Files
          are encrypted at rest with <code>STORAGE_SECRET_KEY</code> and materialized to a
          per-request temp file at download time.
        </CardDescription>
      </CardHeader>
      <CardPanel className="flex flex-col gap-5">
        <FilePickerField
          label="Client ID"
          description="Required for Widevine license requests."
          accept=".bin,application/octet-stream"
          pendingFile={pendingClient}
          onPick={setPendingClient}
          storedStatus={clientStored}
          inputRef={clientInputRef}
        />

        <FilePickerField
          label="Private Key"
          description="PKCS#1 or PKCS#8 PEM."
          accept=".pem,application/x-pem-file"
          pendingFile={pendingPrivateKey}
          onPick={setPendingPrivateKey}
          storedStatus={privateKeyStored}
          inputRef={privateKeyInputRef}
        />

        <Field>
          <FieldLabel>Concurrent Key Acquisitions</FieldLabel>
          <NumberField
            value={concurrentKeyAcquisitions}
            onValueChange={(val) => setConcurrentKeyAcquisitions(val ?? 2)}
            min={1}
            max={8}
          >
            <NumberFieldGroup>
              <NumberFieldDecrement />
              <NumberFieldInput />
              <NumberFieldIncrement />
            </NumberFieldGroup>
          </NumberField>
          <FieldDescription>Parallel Widevine DRM key requests (1-8)</FieldDescription>
        </Field>

        <div className="flex justify-end">
          <Button onClick={handleSave} disabled={isLoading}>
            {isLoading ? 'Saving...' : 'Save Changes'}
          </Button>
        </div>
      </CardPanel>
    </Card>
  );
}
