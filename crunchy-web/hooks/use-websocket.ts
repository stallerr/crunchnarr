'use client';

import { useContext, useEffect, useRef } from 'react';
import { WebSocketContext } from '@/components/providers/websocket-provider';
import type { WsMessageType } from '@/types/websocket';

export function useWebSocket() {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error('useWebSocket must be used within a WebSocketProvider');
  }
  return { isConnected: context.isConnected, send: context.send };
}

export function useWebSocketSubscription(
  type: WsMessageType,
  callback: (data: unknown) => void
) {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error(
      'useWebSocketSubscription must be used within a WebSocketProvider'
    );
  }

  const callbackRef = useRef(callback);
  callbackRef.current = callback;

  useEffect(() => {
    const stableCallback = (data: unknown) => {
      callbackRef.current(data);
    };

    const unsubscribe = context.subscribe(type, stableCallback);
    return unsubscribe;
  }, [type, context]);
}
