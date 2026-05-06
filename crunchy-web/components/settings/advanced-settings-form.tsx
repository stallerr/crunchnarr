'use client';

import { useState } from 'react';
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

type Props = {
  config: AppConfig;
  onSaved: () => void;
};

export function AdvancedSettingsForm({ config, onSaved }: Props) {
  const [cacheRetentionDays, setCacheRetentionDays] = useState(
    config.cache_retention_days ?? 7
  );
  const { execute, isLoading } = useUpdateConfig();

  const handleSave = async () => {
    const { error } = await execute({
      cache_retention_days: cacheRetentionDays,
    });
    if (!error) onSaved();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Advanced Settings</CardTitle>
        <CardDescription>
          Storage and caching configuration.
        </CardDescription>
      </CardHeader>
      <CardPanel className="flex flex-col gap-5">
        <Field>
          <FieldLabel>Cache Retention (days)</FieldLabel>
          <NumberField
            value={cacheRetentionDays}
            onValueChange={(val) => setCacheRetentionDays(val ?? 7)}
            min={1}
            max={365}
          >
            <NumberFieldGroup>
              <NumberFieldDecrement />
              <NumberFieldInput />
              <NumberFieldIncrement />
            </NumberFieldGroup>
          </NumberField>
          <FieldDescription>Segment cache entries older than this are deleted</FieldDescription>
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
