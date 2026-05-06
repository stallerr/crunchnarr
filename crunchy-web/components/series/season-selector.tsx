'use client';

import { cn } from '@/lib/utils';
import type { CRSeason } from '@/types/crunchyroll';

type SeasonSelectorProps = {
  seasons: CRSeason[];
  selectedId: string | null;
  onSelect: (seasonId: string) => void;
};

export function SeasonSelector({ seasons, selectedId, onSelect }: SeasonSelectorProps) {
  if (seasons.length <= 6) {
    return (
      <div className="flex flex-wrap gap-2">
        {seasons.map((season) => (
          <button
            key={season.id}
            onClick={() => onSelect(season.id)}
            className={cn(
              'px-3 py-1.5 rounded-lg text-sm font-medium transition-colors',
              selectedId === season.id
                ? 'bg-primary text-primary-foreground'
                : 'bg-secondary text-secondary-foreground hover:bg-secondary/80'
            )}
          >
            S{season.season_number}
          </button>
        ))}
      </div>
    );
  }

  return (
    <select
      value={selectedId ?? ''}
      onChange={(e) => onSelect(e.target.value)}
      className="h-9 rounded-lg border border-input bg-card px-3 text-sm text-foreground outline-none focus:border-primary"
    >
      {seasons.map((season) => (
        <option key={season.id} value={season.id}>
          Season {season.season_number}
        </option>
      ))}
    </select>
  );
}
