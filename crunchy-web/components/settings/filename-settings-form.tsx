'use client';

import {useState} from 'react';
import {Card, CardHeader, CardTitle, CardDescription, CardPanel} from '@/components/ui/card';
import {Button} from '@/components/ui/button';
import {useUpdateConfig} from '@/hooks/use-config';
import type {AppConfig} from '@/lib/api/calls/config';
import FilenameTemplateBuilder from "@/components/ui/filename-template-builder";

type Props = {
    config: AppConfig;
    onSaved: () => void;
};

const PRESETS: { label: string; description: string; template: string }[] = [
    {
        label: 'Default',
        description: 'My Show - S01E01 - Title',
        template: '{series} - S{season:02}E{episode:02} - {title}',
    },
    {
        label: 'Season folders',
        description: 'My Show/Season 01/My Show - S01E01 - Title',
        template: '{series}/Season {season:02}/{series} - S{season:02}E{episode:02} - {title}',
    },
    {
        label: 'With quality',
        description: 'My Show - S01E01 - Title [1080p]',
        template: '{series} - S{season:02}E{episode:02} - {title} [{quality}]',
    },
    {
        label: 'With audio language',
        description: 'My Show - S01E01 - Title [ja-JP]',
        template: '{series} - S{season:02}E{episode:02} - {title} [{audio}]',
    },
    {
        label: 'Plex-friendly (year + season folders)',
        description: 'My Show (2024)/Season 01/My Show - S01E01 - Title',
        template:
            '{series} ({year})/Season {season:02}/{series} - S{season:02}E{episode:02} - {title}',
    },
];

export function FilenameSettingsForm({config, onSaved}: Props) {
    const [filenameTemplate, setFilenameTemplate] = useState(config.filename_template ?? '');
    // Bumped when a preset is applied so the builder remounts and re-parses
    // its initial template state from the new `value`.
    const [builderVersion, setBuilderVersion] = useState(0);
    const {execute, isLoading} = useUpdateConfig();

    const applyPreset = (template: string) => {
        setFilenameTemplate(template);
        setBuilderVersion((v) => v + 1);
    };

    const handleSave = async () => {
        const {error} = await execute({
            filename_template: filenameTemplate,
        });
        if (!error) onSaved();
    };

    return (
        <Card>
            <CardHeader>
                <CardTitle>Filename Template</CardTitle>
                <CardDescription>
                    Customize the naming format for downloaded files using variables and formatting options. The resulting filename will be generated based on the template you provide, allowing you to organize your downloads in a way that suits your preferences. You can use variables like {'{'}title{'}'}, {'{'}season{'}'}, {'{'}episode{'}'}, and more to create dynamic filenames. For example, a template like &#34;{'{'}title{'}'} - S{'{'}season{'}'}E{'{'}episode{'}'}&#34; would generate filenames like &#34;My Show - S01E01.mkv&#34;. It will then be appended to the output directory set in the Muxing Options.
                </CardDescription>
            </CardHeader>
            <CardPanel className="flex flex-col gap-5">
                <div className="space-y-2.5">
                    <label className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                        Presets
                    </label>
                    <div className="flex flex-wrap gap-2">
                        {PRESETS.map((preset) => {
                            const active = filenameTemplate === preset.template;
                            return (
                                <Button
                                    key={preset.label}
                                    variant={active ? 'default' : 'outline'}
                                    size="sm"
                                    onClick={() => applyPreset(preset.template)}
                                    title={preset.description}
                                >
                                    {preset.label}
                                </Button>
                            );
                        })}
                    </div>
                </div>

                <FilenameTemplateBuilder
                    key={builderVersion}
                    value={filenameTemplate}
                    onChange={setFilenameTemplate}
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
