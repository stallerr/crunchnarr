'use client';

import { BookmarkIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useBookmarks, useToggleBookmark } from '@/hooks/use-bookmarks';
import { cn } from '@/lib/utils';

type BookmarkButtonProps = {
  seriesId: string;
  size?: 'default' | 'sm' | 'icon-sm' | 'icon';
  variant?: 'default' | 'ghost' | 'outline';
  /** When true, hides the "Bookmark"/"Bookmarked" label and shrinks to icon-only. */
  iconOnly?: boolean;
  /** Stop click propagation (used when nested inside a link). */
  stopPropagation?: boolean;
  className?: string;
};

export function BookmarkButton({
  seriesId,
  size,
  variant = 'outline',
  iconOnly = false,
  stopPropagation = false,
  className,
}: BookmarkButtonProps) {
  const { data, refetch } = useBookmarks();
  const { execute, isLoading } = useToggleBookmark();

  const isBookmarked = data?.some((b) => b.series_id === seriesId) ?? false;

  const handleClick = async (e: React.MouseEvent) => {
    if (stopPropagation) {
      e.preventDefault();
      e.stopPropagation();
    }
    const { error } = await execute(seriesId, isBookmarked);
    if (!error) refetch();
  };

  const buttonSize = size ?? (iconOnly ? 'icon-sm' : 'default');

  return (
    <Button
      variant={variant}
      size={buttonSize}
      onClick={handleClick}
      disabled={isLoading}
      aria-label={isBookmarked ? 'Remove bookmark' : 'Add bookmark'}
      aria-pressed={isBookmarked}
      className={className}
    >
      <BookmarkIcon className={cn(isBookmarked && 'fill-current')} />
      {!iconOnly && (isBookmarked ? 'Bookmarked' : 'Bookmark')}
    </Button>
  );
}
