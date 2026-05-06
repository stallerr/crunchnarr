'use client';

import { useState } from 'react';
import { SettingsIcon } from 'lucide-react';
import {
  PagePanel,
  PageHeader,
  PageTitle,
  PageDescription,
} from '@/components/layout/page';
import { DownloadPreferencesForm } from '@/components/settings/download-preferences-form';
import { LanguagePreferencesForm } from '@/components/settings/language-preferences-form';
import { MuxingOptionsForm } from '@/components/settings/muxing-options-form';
import { FilenameSettingsForm } from '@/components/settings/filename-settings-form';
import { AdvancedSettingsForm } from '@/components/settings/advanced-settings-form';
import { ProxySettingsForm } from '@/components/settings/proxy-settings-form';
import { StorageSettingsForm } from '@/components/settings/storage-settings-form';
import { WidevineSettingsForm } from '@/components/settings/widevine-settings-form';
import { SiteSettingsPanel } from '@/components/settings/site-settings-panel';
import { ApiKeysPanel } from '@/components/settings/api-keys-panel';
import { useConfig } from '@/hooks/use-config';
import { cn } from '@/lib/utils';

const TABS = [
  { key: 'downloads', label: 'Downloads' },
  { key: 'site', label: 'Site' },
  { key: 'api-keys', label: 'API Keys' },
] as const;

type TabKey = (typeof TABS)[number]['key'];

export default function SettingsPage() {
  const { data: config, isLoading, error, refetch } = useConfig();
  const [activeTab, setActiveTab] = useState<TabKey>('downloads');

  return (
    <PagePanel>
      <PageHeader>
        <div className="flex items-center gap-2">
          <SettingsIcon className="size-6 text-primary" />
          <PageTitle>Settings</PageTitle>
        </div>
        <PageDescription>
          Configure site preferences, downloads, languages, muxing, and advanced options.
        </PageDescription>
      </PageHeader>

      <div className="flex gap-1 mb-6 border-b">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            type="button"
            onClick={() => setActiveTab(tab.key)}
            className={cn(
              'px-3 py-2 text-sm font-medium transition-colors border-b-2 -mb-px',
              activeTab === tab.key
                ? 'border-primary text-foreground'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            )}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {activeTab === 'site' ? (
        <SiteSettingsPanel />
      ) : activeTab === 'api-keys' ? (
        <ApiKeysPanel />
      ) : isLoading ? (
        <div className="space-y-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="h-48 rounded-2xl border bg-card animate-pulse" />
          ))}
        </div>
      ) : error ? (
        <div className="flex flex-col items-center py-16 text-muted-foreground">
          <p className="text-sm">{error}</p>
        </div>
      ) : config ? (
        <div className="space-y-6">
          <DownloadPreferencesForm config={config} onSaved={refetch} />
          <LanguagePreferencesForm config={config} onSaved={refetch} />
          <FilenameSettingsForm config={config} onSaved={refetch} />
          <MuxingOptionsForm config={config} onSaved={refetch} />
          <WidevineSettingsForm config={config} onSaved={refetch} />
          <ProxySettingsForm config={config} onSaved={refetch} />
          <StorageSettingsForm config={config} onSaved={refetch} />
          <AdvancedSettingsForm config={config} onSaved={refetch} />
        </div>
      ) : null}
    </PagePanel>
  );
}
