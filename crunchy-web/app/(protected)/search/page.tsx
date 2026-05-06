'use client';

import { useState, useEffect, useRef } from 'react';
import { useSearchParams } from 'next/navigation';
import { SearchIcon, LayoutGridIcon, ListIcon } from 'lucide-react';
import { PagePanel, PageHeader, PageTitle, PageDescription } from '@/components/layout/page';
import { SearchInput } from '@/components/search/search-input';
import { SearchResultCard, SearchResultSkeleton } from '@/components/search/search-result-card';
import { useSearch } from '@/hooks/use-search';
import { useCrunchyrollStatus } from '@/hooks/use-crunchyroll';
import { LinkBanner } from '@/components/crunchyroll/link-banner';
import { useDensity } from '@/components/providers/density-provider';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

type ViewLayout = 'grid' | 'row';

export default function SearchPage() {
  const searchParams = useSearchParams();
  const initialQuery = searchParams.get('q') ?? '';
  const [query, setQuery] = useState(initialQuery);
  const [layout, setLayout] = useState<ViewLayout>('row');
  const { results, isLoading, error, search } = useSearch();
  const { isLinked, isLoading: crLoading } = useCrunchyrollStatus();
  const { density } = useDensity();
  const didInitRef = useRef(false);

  // Fire initial search from URL param
  useEffect(() => {
    if (!didInitRef.current && initialQuery) {
      didInitRef.current = true;
      search(initialQuery);
    }
  }, [initialQuery, search]);

  const handleSearch = (value: string) => {
    setQuery(value);
    search(value);

    const url = new URL(window.location.href);
    if (value) {
      url.searchParams.set('q', value);
    } else {
      url.searchParams.delete('q');
    }
    window.history.replaceState(null, '', url.toString());
  };

  // Flatten all items from search results
  const allItems = results.flatMap((r) => r.items);

  if (!crLoading && !isLinked) {
    return (
      <PagePanel>
        <PageHeader>
          <PageTitle>Search</PageTitle>
          <PageDescription>Search the Crunchyroll catalog.</PageDescription>
        </PageHeader>
        <LinkBanner />
      </PagePanel>
    );
  }

  const gridClass = layout === 'grid'
    ? `grid gap-3 ${density === 'compact' ? 'grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-7' : 'grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4'}`
    : `flex flex-col ${density === 'compact' ? 'gap-1' : 'gap-2'}`;

  return (
    <PagePanel>
      <PageHeader>
        <PageTitle>Search</PageTitle>
        <PageDescription>Search the Crunchyroll catalog for anime series.</PageDescription>
      </PageHeader>

      <div className="flex items-center gap-3 mb-6">
        <SearchInput value={query} onChange={handleSearch} className="flex-1" />
        <div className="flex shrink-0 rounded-lg border bg-secondary/40 p-1">
          <Button
            variant="ghost"
            size="icon-xl"
            onClick={() => setLayout('grid')}
            className={cn('size-8 rounded-md', layout === 'grid' && 'bg-primary/15 text-primary')}
          >
            <LayoutGridIcon className="size-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon-xl"
            onClick={() => setLayout('row')}
            className={cn('size-8 rounded-md', layout === 'row' && 'bg-primary/15 text-primary')}
          >
            <ListIcon className="size-4" />
          </Button>
        </div>
      </div>

      {error && (
        <p className="text-sm text-destructive-foreground mb-4">{error}</p>
      )}

      {!query && !isLoading && (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <SearchIcon className="size-12 mb-4 opacity-30" />
          <p className="text-sm">Search for anime series...</p>
        </div>
      )}

      {query && isLoading && (
        <div className={gridClass}>
          {Array.from({ length: 10 }).map((_, i) => (
            <SearchResultSkeleton key={i} layout={layout} />
          ))}
        </div>
      )}

      {query && !isLoading && allItems.length === 0 && !error && (
        <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
          <p className="text-sm">No results found for &quot;{query}&quot;</p>
        </div>
      )}

      {allItems.length > 0 && (
        <div className={gridClass}>
          {allItems.map((item) => (
            <SearchResultCard key={item.id} item={item} layout={layout} />
          ))}
        </div>
      )}
    </PagePanel>
  );
}
