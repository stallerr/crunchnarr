'use client';

import { cn } from '@/lib/utils';
import { formatSpeed, formatEta } from '@/lib/format';

type DownloadSpeedIndicatorProps = {
  speed_bps: number;
  eta_secs: number | null;
  className?: string;
};

export function DownloadSpeedIndicator({
  speed_bps,
  eta_secs,
  className,
}: DownloadSpeedIndicatorProps) {
  const speed = formatSpeed(speed_bps);
  const eta = formatEta(eta_secs);

  return (
    <span
      className={cn(
        'text-xs text-muted-foreground tabular-nums',
        className
      )}
    >
      {speed}
      {eta && (
        <>
          <span className="mx-1 opacity-50">&middot;</span>
          {eta} left
        </>
      )}
    </span>
  );
}
