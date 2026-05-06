'use client';

import { useState } from 'react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel, FieldDescription } from '@/components/ui/field';
import { RadioGroup, Radio } from '@/components/ui/radio-group';
import {
  NumberField,
  NumberFieldGroup,
  NumberFieldInput,
  NumberFieldIncrement,
  NumberFieldDecrement,
} from '@/components/ui/number-field';
import { Input } from '@/components/ui/input';
import { useUpdateConfig } from '@/hooks/use-config';
import type { AppConfig } from '@/lib/api/calls/config';

type Props = {
  config: AppConfig;
  onSaved: () => void;
};

const QUALITY_OPTIONS = [
  { value: 'best', label: 'Best', desc: 'Highest available quality' },
  { value: '1080p', label: '1080p', desc: 'Full HD' },
  { value: '720p', label: '720p', desc: 'HD' },
  { value: '480p', label: '480p', desc: 'Standard' },
  { value: '360p', label: '360p', desc: 'Low' },
];

export function DownloadPreferencesForm({ config, onSaved }: Props) {
  const [quality, setQuality] = useState(config.video_quality ?? 'best');
  const [simultaneousDownloads, setSimultaneousDownloads] = useState(
    config.simultaneous_downloads ?? 2
  );
  const [parallelSegments, setParallelSegments] = useState(
    config.parallel_segments ?? 4
  );
  const [maxSpeed, setMaxSpeed] = useState(
    config.max_speed_kbps !== null && config.max_speed_kbps !== undefined
      ? String(config.max_speed_kbps)
      : ''
  );
  const [retryCount, setRetryCount] = useState(config.retry_count ?? 3);
  const { execute, isLoading } = useUpdateConfig();

  const handleSave = async () => {
    const { error } = await execute({
      video_quality: quality,
      simultaneous_downloads: simultaneousDownloads,
      parallel_segments: parallelSegments,
      max_speed_kbps: maxSpeed.trim() ? Math.max(0, parseInt(maxSpeed)) : null,
      retry_count: retryCount,
    });
    if (!error) onSaved();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Download Preferences</CardTitle>
        <CardDescription>
          Configure video quality and download performance settings.
        </CardDescription>
      </CardHeader>
      <CardPanel className="flex flex-col gap-5">
        <Field>
          <FieldLabel>Video Quality</FieldLabel>
          <RadioGroup
            value={quality}
            onValueChange={setQuality}
            className="grid grid-cols-2 sm:grid-cols-5 gap-2 w-full"
          >
            {QUALITY_OPTIONS.map((opt) => (
              <label
                key={opt.value}
                className="flex items-center gap-2.5 rounded-xl border border-border bg-secondary/40 px-3 py-2.5 cursor-pointer transition-colors hover:bg-secondary/70 has-data-checked:border-primary has-data-checked:bg-primary/8"
              >
                <Radio value={opt.value} />
                <div>
                  <span className="text-sm font-semibold">{opt.label}</span>
                  <p className="text-xs text-muted-foreground">{opt.desc}</p>
                </div>
              </label>
            ))}
          </RadioGroup>
        </Field>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <Field>
            <FieldLabel>Simultaneous Downloads</FieldLabel>
            <NumberField
              value={simultaneousDownloads}
              onValueChange={(val) => setSimultaneousDownloads(val ?? 1)}
              min={1}
              max={10}
            >
              <NumberFieldGroup>
                <NumberFieldDecrement />
                <NumberFieldInput />
                <NumberFieldIncrement />
              </NumberFieldGroup>
            </NumberField>
            <FieldDescription>Number of downloads running at once (1-10)</FieldDescription>
          </Field>

          <Field>
            <FieldLabel>Parallel Segments</FieldLabel>
            <NumberField
              value={parallelSegments}
              onValueChange={(val) => setParallelSegments(val ?? 4)}
              min={1}
              max={32}
            >
              <NumberFieldGroup>
                <NumberFieldDecrement />
                <NumberFieldInput />
                <NumberFieldIncrement />
              </NumberFieldGroup>
            </NumberField>
            <FieldDescription>Concurrent segment downloads per file (1-32)</FieldDescription>
          </Field>

          <Field>
            <FieldLabel>Max Speed (KB/s)</FieldLabel>
            <Input
              type="number"
              min={0}
              placeholder="Unlimited"
              value={maxSpeed}
              onChange={(e) => setMaxSpeed((e.target as HTMLInputElement).value)}
            />
            <FieldDescription>Leave blank for unlimited</FieldDescription>
          </Field>

          <Field>
            <FieldLabel>Retry Count</FieldLabel>
            <NumberField
              value={retryCount}
              onValueChange={(val) => setRetryCount(val ?? 3)}
              min={0}
              max={10}
            >
              <NumberFieldGroup>
                <NumberFieldDecrement />
                <NumberFieldInput />
                <NumberFieldIncrement />
              </NumberFieldGroup>
            </NumberField>
            <FieldDescription>Retries on failed segment downloads (0-10)</FieldDescription>
          </Field>
        </div>

        <div className="flex justify-end">
          <Button onClick={handleSave} disabled={isLoading}>
            {isLoading ? 'Saving...' : 'Save Changes'}
          </Button>
        </div>
      </CardPanel>
    </Card>
  );
}
