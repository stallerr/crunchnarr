'use client';

import { Volume2Icon, SubtitlesIcon } from 'lucide-react';
import { getLanguageName } from '@/lib/languages';

type AudioSubtitleBadgesProps = {
  audioLocale: string;
  subtitleLocales: string[];
};

export function AudioSubtitleBadges({
  audioLocale,
  subtitleLocales,
}: AudioSubtitleBadgesProps) {
  return (
    <div className="space-y-3">
      <div>
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
          <Volume2Icon className="size-3.5" />
          <span>Audio</span>
        </div>
        <span className="inline-flex px-2 py-1 rounded-md bg-secondary text-secondary-foreground text-xs">
          {getLanguageName(audioLocale)}
        </span>
      </div>

      {subtitleLocales.length > 0 && (
        <div>
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground mb-1.5">
            <SubtitlesIcon className="size-3.5" />
            <span>Subtitles</span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {subtitleLocales.map((locale) => (
              <span
                key={locale}
                className="inline-flex px-2 py-1 rounded-md bg-secondary text-secondary-foreground text-xs"
              >
                {getLanguageName(locale)}
              </span>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
