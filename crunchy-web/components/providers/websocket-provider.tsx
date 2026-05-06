'use client';

import {
  createContext,
  ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { BASE_URL } from '@/lib/api/client';
import type { WsMessage, WsMessageType } from '@/types/websocket';

type WsSubscriber = (data: unknown) => void;

type WebSocketContextValue = {
  isConnected: boolean;
  subscribe: (type: WsMessageType, callback: WsSubscriber) => () => void;
  send: (message: object) => void;
};

export const WebSocketContext = createContext<WebSocketContextValue>({
  isConnected: false,
  subscribe: () => () => {},
  send: () => {},
});

const PING_INTERVAL = 30_000;
const MAX_RECONNECT_DELAY = 30_000;

function getWsUrl(token: string): string {
  const wsBase = BASE_URL.replace(/^http/, 'ws');
  return `${wsBase}/ws?token=${token}`;
}

export function WebSocketProvider({ children }: { children: ReactNode }) {
  const { token } = useAuthToken();
  const [isConnected, setIsConnected] = useState(false);

  const wsRef = useRef<WebSocket | null>(null);
  const subscribersRef = useRef<Map<WsMessageType, Set<WsSubscriber>>>(new Map());
  const reconnectAttemptRef = useRef(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const pingTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const intentionalCloseRef = useRef(false);

  const clearTimers = useCallback(() => {
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
    if (pingTimerRef.current) {
      clearInterval(pingTimerRef.current);
      pingTimerRef.current = null;
    }
  }, []);

  const disconnect = useCallback(() => {
    intentionalCloseRef.current = true;
    clearTimers();
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    setIsConnected(false);
  }, [clearTimers]);

  const dispatch = useCallback((message: WsMessage) => {
    const subs = subscribersRef.current.get(message.type);
    if (subs) {
      subs.forEach((cb) => cb(message.data));
    }
  }, []);

  const connect = useCallback(
    (accessToken: string) => {
      if (wsRef.current) {
        wsRef.current.close();
        wsRef.current = null;
      }

      intentionalCloseRef.current = false;
      const url = getWsUrl(accessToken);
      const ws = new WebSocket(url);
      wsRef.current = ws;

      ws.onopen = () => {
        setIsConnected(true);
        reconnectAttemptRef.current = 0;

        pingTimerRef.current = setInterval(() => {
          if (ws.readyState === WebSocket.OPEN) {
            ws.send(JSON.stringify({ type: 'ping' }));
          }
        }, PING_INTERVAL);
      };

      ws.onmessage = (event) => {
        try {
          const message = JSON.parse(event.data) as WsMessage;
          dispatch(message);
        } catch {
          // Ignore malformed messages
        }
      };

      ws.onclose = () => {
        // Only handle if this is still the active connection
        if (wsRef.current !== ws) return;

        setIsConnected(false);
        clearTimers();

        if (!intentionalCloseRef.current) {
          const attempt = reconnectAttemptRef.current;
          const delay = Math.min(1000 * Math.pow(2, attempt), MAX_RECONNECT_DELAY);
          reconnectAttemptRef.current = attempt + 1;

          reconnectTimerRef.current = setTimeout(() => {
            connect(accessToken);
          }, delay);
        }
      };

      ws.onerror = () => {
        // onclose will fire after onerror, which handles reconnection
      };
    },
    [clearTimers, dispatch]
  );

  useEffect(() => {
    if (token) {
      connect(token);
    } else {
      disconnect();
    }

    return () => {
      disconnect();
    };
  }, [token, connect, disconnect]);

  const subscribe = useCallback(
    (type: WsMessageType, callback: WsSubscriber): (() => void) => {
      if (!subscribersRef.current.has(type)) {
        subscribersRef.current.set(type, new Set());
      }
      subscribersRef.current.get(type)!.add(callback);

      return () => {
        const subs = subscribersRef.current.get(type);
        if (subs) {
          subs.delete(callback);
          if (subs.size === 0) {
            subscribersRef.current.delete(type);
          }
        }
      };
    },
    []
  );

  const send = useCallback((message: object) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
    }
  }, []);

  const value = useMemo<WebSocketContextValue>(
    () => ({ isConnected, subscribe, send }),
    [isConnected, subscribe, send]
  );

  return (
    <WebSocketContext.Provider value={value}>
      {children}
    </WebSocketContext.Provider>
  );
}
