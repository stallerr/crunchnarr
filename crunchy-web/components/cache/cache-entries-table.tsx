'use client';

import { cn } from '@/lib/utils';
import { formatBytes, formatRelativeTime } from '@/lib/format';
import type { CacheEntry } from '@/lib/api/calls/cache';

type Props = {
  entries: CacheEntry[];
};

function AgeIndicator({ createdAt }: { createdAt: string }) {
  const ageMs = Date.now() - new Date(createdAt).getTime();
  const ageDays = ageMs / (1000 * 60 * 60 * 24);

  const color =
    ageDays < 1
      ? 'bg-green-500'
      : ageDays < 3
      ? 'bg-yellow-500'
      : 'bg-red-500';

  const label =
    ageDays < 1 ? 'Fresh' : ageDays < 3 ? 'Aging' : 'Old';

  return (
    <span className="flex items-center gap-1.5">
      <span className={cn('size-2 rounded-full shrink-0', color)} />
      <span className="text-xs text-muted-foreground">{label}</span>
    </span>
  );
}

export function CacheEntriesTable({ entries }: Props) {
  if (entries.length === 0) {
    return (
      <div className="flex flex-col items-center py-12 text-muted-foreground">
        <p className="text-sm">No cache entries found.</p>
      </div>
    );
  }

  return (
    <div className="rounded-xl border overflow-hidden">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b bg-muted/50">
            <th className="text-left px-4 py-2.5 text-xs font-medium text-muted-foreground">
              Episode ID
            </th>
            <th className="text-left px-4 py-2.5 text-xs font-medium text-muted-foreground">
              Phase
            </th>
            <th className="text-right px-4 py-2.5 text-xs font-medium text-muted-foreground">
              Size
            </th>
            <th className="text-left px-4 py-2.5 text-xs font-medium text-muted-foreground">
              Created
            </th>
            <th className="text-left px-4 py-2.5 text-xs font-medium text-muted-foreground">
              Age
            </th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry, i) => (
            <tr
              key={`${entry.episode_id}-${entry.phase}-${i}`}
              className="border-b last:border-0 hover:bg-muted/30 transition-colors"
            >
              <td className="px-4 py-2.5 font-mono text-xs truncate max-w-[160px]">
                {entry.episode_id}
              </td>
              <td className="px-4 py-2.5">
                <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-secondary text-secondary-foreground">
                  {entry.phase}
                </span>
              </td>
              <td className="px-4 py-2.5 text-right tabular-nums">
                {formatBytes(entry.size_bytes)}
              </td>
              <td className="px-4 py-2.5 text-muted-foreground text-xs whitespace-nowrap">
                {formatRelativeTime(entry.created_at)}
              </td>
              <td className="px-4 py-2.5">
                <AgeIndicator createdAt={entry.created_at} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
