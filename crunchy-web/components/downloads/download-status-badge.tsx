'use client';

import {
  ClockIcon,
  LoaderCircleIcon,
  CheckCircleIcon,
  XCircleIcon,
  PauseCircleIcon,
  SlashIcon,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { DownloadStatus } from '@/types/downloads';

const STATUS_CONFIG: Record<
  DownloadStatus,
  { label: string; icon: React.ElementType; className: string }
> = {
  pending: {
    label: 'Pending',
    icon: ClockIcon,
    className: 'bg-muted text-muted-foreground',
  },
  active: {
    label: 'Active',
    icon: LoaderCircleIcon,
    className: 'bg-orange-500/15 text-orange-500',
  },
  completed: {
    label: 'Completed',
    icon: CheckCircleIcon,
    className: 'bg-green-500/15 text-green-500',
  },
  failed: {
    label: 'Failed',
    icon: XCircleIcon,
    className: 'bg-red-500/15 text-red-500',
  },
  paused: {
    label: 'Paused',
    icon: PauseCircleIcon,
    className: 'bg-yellow-500/15 text-yellow-500',
  },
  cancelled: {
    label: 'Cancelled',
    icon: SlashIcon,
    className: 'bg-muted text-muted-foreground',
  },
};

type DownloadStatusBadgeProps = {
  status: DownloadStatus;
};

export function DownloadStatusBadge({ status }: DownloadStatusBadgeProps) {
  const config = STATUS_CONFIG[status];
  const Icon = config.icon;

  return (
    <span
      className={cn(
        'inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md text-xs font-medium',
        config.className
      )}
    >
      <Icon
        className={cn(
          'size-3',
          status === 'active' && 'animate-spin'
        )}
      />
      {config.label}
    </span>
  );
}
