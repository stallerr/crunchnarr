'use client';

import Link from 'next/link';
import { CRImage } from '@/components/ui/cr-image';
import { useDensity } from '@/components/providers/density-provider';
import { BookmarkButton } from '@/components/bookmarks/bookmark-button';
import type { CRSearchItem } from '@/types/crunchyroll';
import {cn} from "@/lib/utils.ts";

type SearchResultCardProps = {
  item: CRSearchItem;
  layout?: 'grid' | 'row';
};

export function SearchResultCard({ item, layout = 'grid' }: SearchResultCardProps) {
  const href = item.type === 'series' ? `/series/${item.id}` : `/episodes/${item.id}`;
  const { density } = useDensity();
  const compact = density === 'compact';
  const isSeries = item.type === 'series';

  if (layout === 'row') {
    return (
      <Link
        href={href}
        className={`group relative flex items-center gap-4 overflow-hidden border bg-card hover:border-primary/40 transition-colors ${compact ? 'rounded-lg p-2 gap-3' : 'rounded-xl p-3'}`}
      >
        <div className={`shrink-0 overflow-hidden rounded-md ${compact ? 'w-12 h-16' : 'w-16 h-22'}`}>
          <CRImage
            images={item.images}
            type="tall"
            preferredWidth={100}
            alt={item.title}
            className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
          />
        </div>
        <div className="flex-1 min-w-0">
          <h3 className="font-semibold text-sm line-clamp-1">{item.title}</h3>
          {item.description && (
            <p className={cn("text-xs text-muted-foreground mt-0.5", compact ? "line-clamp-2" : "line-clamp-3")}>
              {item.description}
            </p>
          )}
        </div>
        <span className="shrink-0 text-xs text-muted-foreground capitalize">{item.type.replaceAll('_', ' ')}</span>
        {isSeries && (
          <BookmarkButton
            seriesId={item.id}
            variant="ghost"
            iconOnly
            stopPropagation
            className="shrink-0"
          />
        )}
      </Link>
    );
  }

  return (
    <Link
      href={href}
      className={cn(
        'group relative overflow-hidden border bg-card hover:border-primary/40 transition-colors',
        compact ? 'rounded-lg' : 'rounded-xl'
      )}
    >
      {isSeries && (
        <div className="absolute right-2 top-2 z-10">
          <BookmarkButton
            seriesId={item.id}
            variant="default"
            iconOnly
            stopPropagation
            className="bg-background/80 backdrop-blur-sm"
          />
        </div>
      )}
      <div className="aspect-2/3 overflow-hidden">
        <CRImage
          images={item.images}
          type="tall"
          preferredWidth={compact ? 200 : 300}
          alt={item.title}
          className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
        />
      </div>
      <div className={compact ? "p-2" : "p-3"}>
        <h3 className={compact ? "font-semibold text-xs line-clamp-1" : "font-semibold text-sm line-clamp-1"}>{item.title}</h3>
        {!compact && item.description && (
          <p className="text-xs text-muted-foreground line-clamp-2 mt-1">
            {item.description}
          </p>
        )}
      </div>
    </Link>
  );
}

export function SearchResultSkeleton({ layout = 'grid' }: { layout?: 'grid' | 'row' }) {
  if (layout === 'row') {
    return (
      <div className="flex items-center gap-4 rounded-xl overflow-hidden border bg-card animate-pulse p-3">
        <div className="shrink-0 w-16 h-22 rounded-md bg-muted" />
        <div className="flex-1 space-y-2">
          <div className="h-4 w-1/3 bg-muted rounded" />
          <div className="h-3 w-2/3 bg-muted rounded" />
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-xl overflow-hidden border bg-card animate-pulse">
      <div className="aspect-2/3 bg-muted" />
      <div className="p-3 space-y-2">
        <div className="h-4 w-3/4 bg-muted rounded" />
        <div className="h-3 w-full bg-muted rounded" />
      </div>
    </div>
  );
}
