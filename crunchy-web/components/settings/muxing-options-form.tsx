'use client';

import { useState } from 'react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Field, FieldLabel, FieldDescription } from '@/components/ui/field';
import { ToggleOption } from '@/components/ui/toggle-option';
import { RadioGroup, Radio } from '@/components/ui/radio-group';
import {
  Select,
  SelectTrigger,
  SelectValue,
  SelectContent,
  SelectItem,
} from '@/components/ui/select';
import { SubtitlesIcon, MicIcon } from 'lucide-react';
import { useUpdateConfig } from '@/hooks/use-config';
import { LANGUAGE_NAMES } from '@/lib/languages';
import type { AppConfig } from '@/lib/api/calls/config';

type Props = {
  config: AppConfig;
  onSaved: () => void;
};

const LANGUAGE_OPTIONS = [
  { value: '', label: 'Original / Auto' },
  ...Object.entries(LANGUAGE_NAMES).map(([code, name]) => ({ value: code, label: name })),
];

export function MuxingOptionsForm({ config, onSaved }: Props) {
  const [outputFormat, setOutputFormat] = useState(config.output_format ?? 'mkv');
  const [embedSubtitles, setEmbedSubtitles] = useState(config.embed_subtitles ?? true);
  const [defaultAudioTrack, setDefaultAudioTrack] = useState(config.default_audio_track ?? '');
  const [defaultSubtitleTrack, setDefaultSubtitleTrack] = useState(config.default_subtitle_track ?? '');
  const [preferSignsSongs, setPreferSignsSongs] = useState(config.prefer_signs_songs ?? false);
  const [outputDir, setOutputDir] = useState(config.output_dir ?? '');
  const { execute, isLoading } = useUpdateConfig();

  const handleSave = async () => {
    const { error } = await execute({
      output_format: outputFormat,
      embed_subtitles: embedSubtitles,
      default_audio_track: defaultAudioTrack,
      default_subtitle_track: defaultSubtitleTrack,
      prefer_signs_songs: preferSignsSongs,
      output_dir: outputDir.trim(),
    });
    if (!error) onSaved();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Muxing Options</CardTitle>
        <CardDescription>
          Configure how downloaded tracks are combined into the final file.
        </CardDescription>
      </CardHeader>
      <CardPanel className="flex flex-col gap-5">
        <Field>
          <FieldLabel>Output Format</FieldLabel>
          <RadioGroup
            value={outputFormat}
            onValueChange={setOutputFormat}
            className="flex flex-row gap-3"
          >
            <label className="flex flex-1 items-center gap-3 rounded-xl border border-border bg-secondary/40 px-4 py-3 cursor-pointer transition-colors hover:bg-secondary/70 has-data-checked:border-primary has-data-checked:bg-primary/8">
              <Radio value="mkv" />
              <div>
                <span className="text-sm font-semibold">MKV <span className="text-xs font-medium text-primary">Recommended</span></span>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Matroska — best multi-track support
                </p>
              </div>
            </label>
            <label className="flex flex-1 items-center gap-3 rounded-xl border border-border bg-secondary/40 px-4 py-3 cursor-pointer transition-colors hover:bg-secondary/70 has-data-checked:border-primary has-data-checked:bg-primary/8">
              <Radio value="mp4" />
              <div>
                <span className="text-sm font-semibold">MP4</span>
                <p className="text-xs text-muted-foreground mt-0.5">
                  MPEG-4 — widest device compatibility
                </p>
              </div>
            </label>
          </RadioGroup>
        </Field>

        <ToggleOption
          icon={SubtitlesIcon}
          title="Embed Subtitles"
          description="Mux subtitle tracks into the output container"
          recommended
          checked={embedSubtitles}
          onCheckedChange={setEmbedSubtitles}
          className="rounded-lg border"
        />

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <Field>
            <FieldLabel>Default Audio Track</FieldLabel>
            <Select value={defaultAudioTrack} onValueChange={(val) => setDefaultAudioTrack(val ?? '')}>
              <SelectTrigger>
                <SelectValue placeholder="Original / Auto">
                    {defaultAudioTrack
                        ? LANGUAGE_OPTIONS.find((opt) => opt.value === defaultAudioTrack)?.label
                        : 'Original / Auto'}
                </SelectValue>
              </SelectTrigger>
              <SelectContent>
                {LANGUAGE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <FieldDescription>Track flagged as default in the container</FieldDescription>
          </Field>

          <Field>
            <FieldLabel>Default Subtitle Track</FieldLabel>
            <Select value={defaultSubtitleTrack} onValueChange={(val) => setDefaultSubtitleTrack(val ?? '')}>
              <SelectTrigger>
                <SelectValue placeholder="Original / Auto">
                  {defaultSubtitleTrack
                    ? LANGUAGE_OPTIONS.find((opt) => opt.value === defaultSubtitleTrack)?.label
                    : 'Original / Auto'}
                </SelectValue>
              </SelectTrigger>
              <SelectContent>
                {LANGUAGE_OPTIONS.map((opt) => (
                  <SelectItem key={opt.value} value={opt.value}>
                    {opt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <FieldDescription>Track flagged as default in the container</FieldDescription>
          </Field>
        </div>

        <ToggleOption
          icon={MicIcon}
          title="Force Signs & Songs"
          description="Add forced flag to subtitle track, which match the language of the default subtitle, containing signs or songs when detected, so they show up in players as 'forced subtitles'"
          checked={preferSignsSongs}
          onCheckedChange={setPreferSignsSongs}
          className="rounded-lg border"
        />

        <Field>
          <FieldLabel>Output Directory</FieldLabel>
          <Input
            value={outputDir}
            onChange={(e) => setOutputDir((e.target as HTMLInputElement).value)}
            placeholder="/downloads"
          />
          <FieldDescription>
            Absolute path where completed downloads are saved
          </FieldDescription>
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
