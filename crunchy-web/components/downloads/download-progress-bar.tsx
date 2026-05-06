'use client';

import { cn } from '@/lib/utils';

type DownloadProgressBarProps = {
  /** 0-100 percentage. Pass null for indeterminate. */
  progress: number | null;
  /** Optional phase label (e.g., "Video", "Audio (ja-JP)") */
  phase?: string;
  /** Current step number (1-based) */
  currentStep?: number;
  /** Total number of steps */
  totalSteps?: number;
  /** Number of completed segments */
  completedSegments?: number;
  /** Total number of segments */
  totalSegments?: number;
  className?: string;
};

export function DownloadProgressBar({
  progress,
  phase,
  currentStep,
  totalSteps,
  completedSegments,
  totalSegments,
  className,
}: DownloadProgressBarProps) {
  const percentage = progress ?? 0;
  const isIndeterminate = progress === null;
  const hasStepInfo = currentStep != null && totalSteps != null;
  const hasSegmentInfo =
    completedSegments != null && totalSegments != null && totalSegments > 0;

  return (
    <div className={cn('flex flex-col gap-1', className)}>
      {(hasStepInfo || phase) && (
        <div className="flex items-center gap-1.5 text-[11px] leading-none">
          {hasStepInfo && (
            <span className="font-mono text-muted-foreground">
              [{currentStep}/{totalSteps}]
            </span>
          )}
          {phase && (
            <span className="font-medium text-foreground">{phase}</span>
          )}
          {hasSegmentInfo && (
            <span className="text-muted-foreground">
              — {completedSegments}/{totalSegments} segments
            </span>
          )}
        </div>
      )}
      <div className="flex items-center gap-2">
        <div className="flex-1 h-1.5 rounded-full bg-muted overflow-hidden">
          <div
            className={cn(
              'h-full rounded-full bg-orange-500 transition-all duration-300',
              isIndeterminate && 'animate-pulse w-full'
            )}
            style={
              isIndeterminate
                ? undefined
                : { width: `${Math.min(100, percentage)}%` }
            }
          />
        </div>
        {!isIndeterminate && (
          <span className="text-xs text-muted-foreground tabular-nums w-10 text-right">
            {Math.round(percentage)}%
          </span>
        )}
      </div>
    </div>
  );
}
