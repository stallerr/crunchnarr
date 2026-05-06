'use client';

import { useWebSocket } from '@/hooks/use-websocket';
import { cn } from '@/lib/utils';

export function ConnectionStatus() {
  const { isConnected } = useWebSocket();

  return (
    <div className="flex items-center gap-2 px-2 py-1">
      <div
        className={cn(
          'size-2 rounded-full',
          isConnected ? 'bg-green-500' : 'bg-red-500'
        )}
      />
      <span className="text-xs text-muted-foreground">
        {isConnected ? 'Connected' : 'Disconnected'}
      </span>
    </div>
  );
}
