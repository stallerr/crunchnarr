'use client';

import {
  DownloadIcon,
  CheckCircle2Icon,
  ActivityIcon,
  LoaderCircleIcon,
} from 'lucide-react';
import type { DownloadCounts } from '@/types/downloads';

type MetricCardsProps = {
  counts: DownloadCounts | null;
};

export function MetricCards({ counts }: MetricCardsProps) {
  const activeCount = counts?.active ?? 0;
  const totalCompleted = counts?.completed ?? 0;

  const metrics = [
    {
      label: 'Active Downloads',
      value: activeCount,
      icon: activeCount > 0 ? LoaderCircleIcon : DownloadIcon,
      iconSpin: activeCount > 0,
    },
    {
      label: 'Total Downloaded',
      value: totalCompleted,
      icon: CheckCircle2Icon,
      iconSpin: false,
    },
    {
      label: 'Total',
      value: counts?.all ?? 0,
      icon: ActivityIcon,
      iconSpin: false,
    },
  ];

  return (
    <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
      {metrics.map((metric) => {
        const Icon = metric.icon;
        return (
          <div key={metric.label} className="rounded-xl border bg-card p-4">
            <div className="flex items-center gap-3">
              <div className="rounded-lg bg-primary/10 p-2">
                <Icon
                  className={`size-4 text-primary ${metric.iconSpin ? 'animate-spin' : ''}`}
                />
              </div>
              <div>
                <p className="text-sm text-muted-foreground">{metric.label}</p>
                <p className="text-2xl font-semibold tabular-nums">
                  {metric.value}
                </p>
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
