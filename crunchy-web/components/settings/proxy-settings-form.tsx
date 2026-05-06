'use client';

import { useState } from 'react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Field, FieldLabel, FieldDescription } from '@/components/ui/field';
import { ToggleOption } from '@/components/ui/toggle-option';
import { GlobeIcon } from 'lucide-react';
import { useUpdateConfig } from '@/hooks/use-config';
import type { AppConfig } from '@/lib/api/calls/config';

type Props = {
  config: AppConfig;
  onSaved: () => void;
};

export function ProxySettingsForm({ config, onSaved }: Props) {
  const [proxyEnabled, setProxyEnabled] = useState(config.proxy_enabled ?? false);
  const [proxyUrl, setProxyUrl] = useState(config.proxy_url ?? '');
  const { execute, isLoading } = useUpdateConfig();

  const handleSave = async () => {
    const { error } = await execute({
      proxy_enabled: proxyEnabled,
      proxy_url: proxyUrl.trim(),
    });
    if (!error) onSaved();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Proxy</CardTitle>
        <CardDescription>
          Route requests through a proxy server.
        </CardDescription>
      </CardHeader>
      <CardPanel className="flex flex-col gap-5">
        <ToggleOption
          icon={GlobeIcon}
          title="Enable Proxy"
          description="Route all API requests through a proxy server"
          checked={proxyEnabled}
          onCheckedChange={setProxyEnabled}
          className="rounded-lg border"
        />

        {proxyEnabled && (
          <Field>
            <FieldLabel>Proxy URL</FieldLabel>
            <Input
              value={proxyUrl}
              onChange={(e) => setProxyUrl((e.target as HTMLInputElement).value)}
              placeholder="http://user:pass@proxy.example.com:8080"
            />
            <FieldDescription>HTTP/HTTPS/SOCKS5 proxy URL</FieldDescription>
          </Field>
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
