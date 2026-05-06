'use client';

import { useState } from 'react';
import Link from 'next/link';
import { ImageOffIcon } from 'lucide-react';
import { CRImage } from '@/components/ui/cr-image';
import { Input } from '@/components/ui/input';
import { BookmarkButton } from './bookmark-button';
import { useUpdateBookmarkNote } from '@/hooks/use-bookmarks';
import type { BookmarkItem } from '@/lib/api/calls/bookmarks';

type Props = {
  item: BookmarkItem;
  onChanged: () => void;
};

export function BookmarkCard({ item, onChanged }: Props) {
  const [note, setNote] = useState(item.note);
  const { execute: saveNote, isLoading: savingNote } = useUpdateBookmarkNote();

  const handleNoteBlur = async () => {
    if (note === item.note) return;
    const { error } = await saveNote(item.series_id, note);
    if (!error) onChanged();
  };

  const title = item.series?.title ?? 'Series unavailable';
  const href = item.series ? `/series/${item.series_id}` : undefined;

  return (
    <div className="group relative rounded-xl overflow-hidden border bg-card hover:border-primary/40 transition-colors flex flex-col">
      <div className="absolute right-2 top-2 z-10">
        <BookmarkButton
          seriesId={item.series_id}
          variant="default"
          iconOnly
          stopPropagation
          className="bg-background/80 backdrop-blur-sm"
        />
      </div>

      {href ? (
        <Link href={href} className="block aspect-2/3 overflow-hidden">
          {item.series ? (
            <CRImage
              images={item.series.images}
              type="tall"
              preferredWidth={300}
              alt={title}
              className="w-full h-full object-cover group-hover:scale-105 transition-transform duration-300"
            />
          ) : null}
        </Link>
      ) : (
        <div className="aspect-2/3 flex items-center justify-center bg-muted/40 text-muted-foreground">
          <ImageOffIcon className="size-10 opacity-50" />
        </div>
      )}

      <div className="p-3 flex flex-col gap-2 flex-1">
        {href ? (
          <Link
            href={href}
            className="font-semibold text-sm line-clamp-1 hover:underline"
          >
            {title}
          </Link>
        ) : (
          <span className="font-semibold text-sm line-clamp-1 text-muted-foreground">
            {title}
          </span>
        )}

        {item.series?.description && (
          <p className="text-xs text-muted-foreground line-clamp-2">
            {item.series.description}
          </p>
        )}

        <div className="mt-auto pt-2">
          <Input
            value={note}
            onChange={(e) => setNote(e.target.value)}
            onBlur={handleNoteBlur}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                (e.target as HTMLInputElement).blur();
              }
            }}
            placeholder="Add a note…"
            disabled={savingNote || !item.series}
            size="sm"
            maxLength={1000}
          />
        </div>
      </div>
    </div>
  );
}
