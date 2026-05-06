'use client';

import {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import axios from 'axios';

type AuthTokenContextValue = {
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  checkSession: () => Promise<boolean>;
  refreshSession: () => Promise<boolean>;
  getToken: () => Promise<string | null>;
};

const AuthTokenContext = createContext<AuthTokenContextValue>({
  token: null,
  isAuthenticated: false,
  isLoading: true,
  checkSession: async () => false,
  refreshSession: async () => false,
  getToken: async () => null,
});

const TOKEN_REFRESH_INTERVAL = 50 * 60 * 1000; // 50 minutes

export function AuthTokenProvider({ children }: { children: ReactNode }) {
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const tokenRef = useRef<string | null>(null);
  const pendingTokenRequest = useRef<Promise<string | null> | null>(null);

  const fetchToken = useCallback(async (): Promise<string | null> => {
    try {
      const response = await axios.get('/api/auth/token');
      const accessToken = response.data.access_token;
      tokenRef.current = accessToken;
      setToken(accessToken);
      return accessToken;
    } catch {
      tokenRef.current = null;
      setToken(null);
      return null;
    }
  }, []);

  const getToken = useCallback(async (): Promise<string | null> => {
    // Return cached token if available
    if (tokenRef.current) {
      return tokenRef.current;
    }
    // Deduplicate concurrent calls
    if (pendingTokenRequest.current) {
      return pendingTokenRequest.current;
    }
    pendingTokenRequest.current = fetchToken().finally(() => {
      pendingTokenRequest.current = null;
    });
    return pendingTokenRequest.current;
  }, [fetchToken]);

  const checkSession = useCallback(async (): Promise<boolean> => {
    try {
      const response = await axios.get('/api/auth/session');
      if (response.data.authenticated) {
        await getToken();
        return true;
      }
      tokenRef.current = null;
      setToken(null);
      return false;
    } catch {
      tokenRef.current = null;
      setToken(null);
      return false;
    }
  }, [getToken]);

  const refreshSession = useCallback(async (): Promise<boolean> => {
    try {
      await axios.post('/api/auth/token/refresh');
      // Clear cached token so fetchToken gets the new one
      tokenRef.current = null;
      const newToken = await getToken();
      return newToken !== null;
    } catch {
      tokenRef.current = null;
      setToken(null);
      return false;
    }
  }, [getToken]);

  // Initial session check
  useEffect(() => {
    checkSession().finally(() => setIsLoading(false));
  }, [checkSession]);

  // Auto-refresh interval
  useEffect(() => {
    if (!token) return;

    const interval = setInterval(async () => {
      await refreshSession();
    }, TOKEN_REFRESH_INTERVAL);

    return () => clearInterval(interval);
  }, [token, refreshSession]);

  const value = useMemo<AuthTokenContextValue>(
    () => ({
      token,
      isAuthenticated: token !== null,
      isLoading,
      checkSession,
      refreshSession,
      getToken,
    }),
    [token, isLoading, checkSession, refreshSession, getToken]
  );

  return (
    <AuthTokenContext.Provider value={value}>
      {children}
    </AuthTokenContext.Provider>
  );
}

export function useAuthToken() {
  const context = useContext(AuthTokenContext);
  if (!context) {
    throw new Error('useAuthToken must be used within an AuthTokenProvider');
  }
  return context;
}
