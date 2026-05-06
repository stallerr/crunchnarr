'use client';

import { CheckIcon, ShieldOffIcon } from 'lucide-react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Field, FieldLabel } from '@/components/ui/field';
import { RadioGroup, Radio } from '@/components/ui/radio-group';
import { ToggleOption } from '@/components/ui/toggle-option';
import { useAccentColor, ACCENT_COLORS } from '@/components/providers/accent-color-provider';
import { useNavigationMode } from '@/components/providers/navigation-provider';
import { useDensity } from '@/components/providers/density-provider';
import { useConfirmCancel } from '@/components/providers/confirm-cancel-provider';
import { cn } from '@/lib/utils';

const NAV_OPTIONS = [
  { value: 'sidebar', label: 'Sidebar', description: 'Fixed sidebar navigation' },
  { value: 'dock', label: 'Dock', description: 'Floating dock at the bottom' },
  { value: 'both', label: 'Both', description: 'Show sidebar and dock together' },
] as const;

export function SiteSettingsPanel() {
  const { accentColor, setAccentColor } = useAccentColor();
  const { mode, setMode } = useNavigationMode();
  const { density, setDensity } = useDensity();
  const { skipConfirm, setSkipConfirm } = useConfirmCancel();

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Appearance</CardTitle>
          <CardDescription>Choose your primary accent color.</CardDescription>
        </CardHeader>
        <CardPanel>
          <Field>
            <FieldLabel>Accent Color</FieldLabel>
            <div className="flex flex-wrap gap-3">
              {ACCENT_COLORS.map((color) => {
                const selected = accentColor === color.value;
                return (
                  <button
                    key={color.name}
                    type="button"
                    onClick={() => setAccentColor(color.value)}
                    className={cn(
                      'group relative size-10 rounded-full border-2 transition-all cursor-pointer',
                      selected
                        ? 'border-foreground scale-110'
                        : 'border-transparent hover:scale-105'
                    )}
                    style={{ backgroundColor: color.value }}
                    title={color.name}
                  >
                    {selected && (
                      <CheckIcon className="absolute inset-0 m-auto size-5 text-white drop-shadow-md" />
                    )}
                  </button>
                );
              })}
            </div>
          </Field>
        </CardPanel>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Layout</CardTitle>
          <CardDescription>Adjust spacing and density of lists and tables.</CardDescription>
        </CardHeader>
        <CardPanel>
          <Field>
            <FieldLabel>Density</FieldLabel>
            <RadioGroup
              value={density}
              onValueChange={(val) => setDensity(val as 'compact' | 'comfortable')}
              className="flex flex-row gap-3"
            >
              <label className="flex flex-1 items-center gap-3 rounded-xl border border-border bg-secondary/40 px-4 py-3 cursor-pointer transition-colors hover:bg-secondary/70 has-data-checked:border-primary has-data-checked:bg-primary/8">
                <Radio value="compact" />
                <div>
                  <span className="text-sm font-semibold">Compact</span>
                  <p className="text-xs text-muted-foreground mt-0.5">Smaller thumbnails and tighter spacing</p>
                </div>
              </label>
              <label className="flex flex-1 items-center gap-3 rounded-xl border border-border bg-secondary/40 px-4 py-3 cursor-pointer transition-colors hover:bg-secondary/70 has-data-checked:border-primary has-data-checked:bg-primary/8">
                <Radio value="comfortable" />
                <div>
                  <span className="text-sm font-semibold">Comfortable</span>
                  <p className="text-xs text-muted-foreground mt-0.5">Larger thumbnails and more breathing room</p>
                </div>
              </label>
            </RadioGroup>
          </Field>
        </CardPanel>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Navigation</CardTitle>
          <CardDescription>Choose your preferred navigation style.</CardDescription>
        </CardHeader>
        <CardPanel>
          <Field>
            <FieldLabel>Navigation Mode</FieldLabel>
            <RadioGroup
              value={mode}
              onValueChange={(val) => setMode(val as 'sidebar' | 'dock' | 'both')}
              className="flex flex-row gap-3"
            >
              {NAV_OPTIONS.map((opt) => (
                <label
                  key={opt.value}
                  className="flex flex-1 items-center gap-3 rounded-xl border border-border bg-secondary/40 px-4 py-3 cursor-pointer transition-colors hover:bg-secondary/70 has-data-checked:border-primary has-data-checked:bg-primary/8"
                >
                  <Radio value={opt.value} />
                  <div>
                    <span className="text-sm font-semibold">{opt.label}</span>
                    <p className="text-xs text-muted-foreground mt-0.5">{opt.description}</p>
                  </div>
                </label>
              ))}
            </RadioGroup>
          </Field>
        </CardPanel>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Downloads</CardTitle>
          <CardDescription>Customize download behavior.</CardDescription>
        </CardHeader>
        <CardPanel>
          <ToggleOption
            icon={ShieldOffIcon}
            title="Skip Cancel Confirmation"
            description="Cancel downloads immediately without a confirmation dialog"
            checked={skipConfirm}
            onCheckedChange={setSkipConfirm}
            className="rounded-lg border"
          />
        </CardPanel>
      </Card>
    </div>
  );
}
