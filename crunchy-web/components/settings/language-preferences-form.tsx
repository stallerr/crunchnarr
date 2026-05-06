'use client';

import { useState } from 'react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Field, FieldLabel } from '@/components/ui/field';
import { MultiSelect } from '@/components/ui/multi-select';
import { ToggleOption } from '@/components/ui/toggle-option';
import { ConfirmDialog } from '@/components/ui/confirm-dialog';
import { CaptionsIcon } from 'lucide-react';
import { useUpdateConfig } from '@/hooks/use-config';
import { LANGUAGE_NAMES } from '@/lib/languages';
import type { AppConfig } from '@/lib/api/calls/config';

type Props = {
  config: AppConfig;
  onSaved: () => void;
};

const LANGUAGE_OPTIONS = Object.entries(LANGUAGE_NAMES).map(([code, name]) => ({
  value: code,
  label: name,
}));

export function LanguagePreferencesForm({ config, onSaved }: Props) {
  const [audioLanguages, setAudioLanguages] = useState(config.audio_languages ?? []);
  const [subtitleLanguages, setSubtitleLanguages] = useState(config.subtitle_languages ?? []);
  const [closedCaptions, setClosedCaptions] = useState(config.closed_captions ?? false);
  const [ccWarningOpen, setCcWarningOpen] = useState(false);
  const { execute, isLoading } = useUpdateConfig();

  const handleSave = async () => {
    const { error } = await execute({
      audio_languages: audioLanguages,
      subtitle_languages: subtitleLanguages,
      closed_captions: closedCaptions,
    });
    if (!error) onSaved();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Language Preferences</CardTitle>
        <CardDescription>
          Choose which audio and subtitle languages to download by default.
        </CardDescription>
      </CardHeader>
      <CardPanel className="flex flex-col gap-5">
        <Field>
          <FieldLabel>Audio Languages</FieldLabel>
          <MultiSelect
            options={LANGUAGE_OPTIONS}
            value={audioLanguages}
            onValueChange={setAudioLanguages}
            placeholder="Select audio languages"
            searchPlaceholder="Search languages..."
          />
        </Field>

        <Field>
          <FieldLabel>Subtitle Languages</FieldLabel>
          <MultiSelect
            options={LANGUAGE_OPTIONS}
            value={subtitleLanguages}
            onValueChange={setSubtitleLanguages}
            placeholder="Select subtitle languages"
            searchPlaceholder="Search languages..."
          />
        </Field>

        <ToggleOption
          icon={CaptionsIcon}
          title="Include Closed Captions"
          description="Download CC/SDH subtitle tracks when available"
          experimental
          checked={closedCaptions}
          onCheckedChange={(checked) => {
            if (checked) {
              setCcWarningOpen(true);
            } else {
              setClosedCaptions(false);
            }
          }}
          className="rounded-lg border"
        />

        <ConfirmDialog
          open={ccWarningOpen}
          onOpenChange={setCcWarningOpen}
          title="Experimental Feature"
          description="Closed captions support is still in development and may be unstable. Downloads could fail or produce incomplete subtitle tracks. Are you sure you want to enable this?"
          confirmLabel="Enable Anyway"
          cancelLabel="Dismiss"
          onConfirm={() => {
            setClosedCaptions(true);
            setCcWarningOpen(false);
          }}
        />

        <div className="flex justify-end">
          <Button onClick={handleSave} disabled={isLoading}>
            {isLoading ? 'Saving...' : 'Save Changes'}
          </Button>
        </div>
      </CardPanel>
    </Card>
  );
}
