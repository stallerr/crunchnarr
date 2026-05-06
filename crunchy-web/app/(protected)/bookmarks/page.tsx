'use client';

import Link from 'next/link';
import { BookmarkIcon, SearchIcon } from 'lucide-react';
import {
  PagePanel,
  PageHeader,
  PageTitle,
  PageDescription,
} from '@/components/layout/page';
import { BookmarkCard } from '@/components/bookmarks/bookmark-card';
import { Button } from '@/components/ui/button';
import { useBookmarks } from '@/hooks/use-bookmarks';

export default function BookmarksPage() {
  const { data, isLoading, error, refetch } = useBookmarks();

  return (
    <PagePanel>
      <PageHeader>
        <div className="flex items-center gap-2">
          <BookmarkIcon className="size-6 text-primary" />
          <PageTitle>Bookmarks</PageTitle>
        </div>
        <PageDescription>
          Your saved series. Click a card to open the series, edit the note inline.
        </PageDescription>
      </PageHeader>

      {isLoading ? (
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="rounded-xl overflow-hidden border bg-card animate-pulse"
            >
              <div className="aspect-2/3 bg-muted" />
              <div className="p-3 space-y-2">
                <div className="h-4 w-3/4 bg-muted rounded" />
                <div className="h-3 w-full bg-muted rounded" />
              </div>
            </div>
          ))}
        </div>
      ) : error ? (
        <div className="flex flex-col items-center py-16 text-muted-foreground">
          <p className="text-sm">{error}</p>
        </div>
      ) : !data || data.length === 0 ? (
        <div className="flex flex-col items-center gap-4 py-20 text-muted-foreground">
          <BookmarkIcon className="size-12 opacity-50" />
          <p className="text-sm">No bookmarks yet.</p>
          <Button variant="outline" render={<Link href="/search" />}>
            <SearchIcon />
            Find something to watch
          </Button>
        </div>
      ) : (
        <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
          {data.map((item) => (
            <BookmarkCard key={item.series_id} item={item} onChanged={refetch} />
          ))}
        </div>
      )}
    </PagePanel>
  );
}
