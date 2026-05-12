'use client';

import { useState } from 'react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Field, FieldLabel, FieldDescription } from '@/components/ui/field';
import { Switch } from '@/components/ui/switch';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';
import { useUpdateConfig } from '@/hooks/use-config';
import type { AppConfig, StorageConfig } from '@/lib/api/calls/config';

type Props = {
  config: AppConfig;
  onSaved: () => void;
};

const DEFAULTS: StorageConfig = {
  kind: 'local',
  bucket: '',
  region: '',
  endpoint: '',
  prefix: '',
  access_key_id: '',
  secret_access_key: '',
  force_path_style: false,
};

export function StorageSettingsForm({ config, onSaved }: Props) {
  const initial = { ...DEFAULTS, ...(config.storage ?? {}) };
  const [storage, setStorage] = useState<StorageConfig>(initial);
  const { execute, isLoading } = useUpdateConfig();

  const update = <K extends keyof StorageConfig>(key: K, value: StorageConfig[K]) =>
    setStorage((prev) => ({ ...prev, [key]: value }));

  const handleSave = async () => {
    const { error } = await execute({ storage });
    if (!error) onSaved();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Storage</CardTitle>
        <CardDescription>
          Where finished downloads land. Use Local for the filesystem on this server, or S3 for any
          S3-compatible bucket (AWS S3, MinIO, Cloudflare R2, Backblaze B2).
        </CardDescription>
      </CardHeader>
      <CardPanel className="flex flex-col gap-5">
        <Field>
          <FieldLabel>Backend</FieldLabel>
          <Select
            value={storage.kind}
            onValueChange={(val) => update('kind', (val as StorageConfig['kind']) ?? 'local')}
          >
            <SelectTrigger>
              <SelectValue placeholder="Local filesystem">
                {storage.kind === 's3' ? 'S3-compatible bucket' : 'Local filesystem'}
              </SelectValue>
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="local">Local filesystem</SelectItem>
              <SelectItem value="s3">S3-compatible bucket</SelectItem>
            </SelectContent>
          </Select>
        </Field>

        {storage.kind === 'local' ? (
          <p className="text-muted-foreground text-xs">
            Local files land at the <strong>Output Directory</strong> configured in <em>Muxing Options</em>.
          </p>
        ) : (
          <>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <Field>
                <FieldLabel>Bucket</FieldLabel>
                <Input
                  value={storage.bucket}
                  onChange={(e) => update('bucket', (e.target as HTMLInputElement).value)}
                  placeholder="my-anime-bucket"
                />
              </Field>
              <Field>
                <FieldLabel>Region</FieldLabel>
                <Input
                  value={storage.region}
                  onChange={(e) => update('region', (e.target as HTMLInputElement).value)}
                  placeholder="us-east-1"
                />
              </Field>
            </div>

            <Field>
              <FieldLabel>Endpoint</FieldLabel>
              <Input
                value={storage.endpoint}
                onChange={(e) => update('endpoint', (e.target as HTMLInputElement).value)}
                placeholder="(blank for AWS) or https://minio.local:9000"
              />
              <FieldDescription>
                Leave blank for AWS. Set for MinIO, R2, B2, or any other S3-compatible host.
              </FieldDescription>
            </Field>

            <Field>
              <FieldLabel>Key prefix</FieldLabel>
              <Input
                value={storage.prefix}
                onChange={(e) => update('prefix', (e.target as HTMLInputElement).value)}
                placeholder="shows/"
              />
              <FieldDescription>Optional prefix prepended to every object key.</FieldDescription>
            </Field>

            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <Field>
                <FieldLabel>Access Key ID</FieldLabel>
                <Input
                  value={storage.access_key_id}
                  onChange={(e) => update('access_key_id', (e.target as HTMLInputElement).value)}
                  placeholder="AKIA..."
                />
              </Field>
              <Field>
                <FieldLabel>Secret Access Key</FieldLabel>
                <Input
                  type="password"
                  value={storage.secret_access_key}
                  onChange={(e) =>
                    update('secret_access_key', (e.target as HTMLInputElement).value)
                  }
                  placeholder="********"
                />
                <FieldDescription>
                  The server returns <code>********</code> for an existing secret. Leave the
                  placeholder to keep it; type a new value to replace it.
                </FieldDescription>
              </Field>
            </div>

            <Field>
              <div className="flex items-center justify-between">
                <div>
                  <FieldLabel>Force path-style URLs</FieldLabel>
                  <FieldDescription>
                    Required for MinIO and most non-AWS providers.
                  </FieldDescription>
                </div>
                <Switch
                  checked={storage.force_path_style}
                  onCheckedChange={(checked) => update('force_path_style', !!checked)}
                />
              </div>
            </Field>
          </>
        )}

        <div className="flex justify-end">
          <Button onClick={handleSave} disabled={isLoading}>
            {isLoading ? 'Saving...' : 'Save Changes'}
          </Button>
        </div>
      </CardPanel>
    </Card>
  );
}
