'use client';

import { createContext, useContext, useEffect, useState, useMemo, type ReactNode } from 'react';

type NavigationMode = 'sidebar' | 'dock' | 'both';

type NavigationModeState = {
  mode: NavigationMode;
  setMode: (mode: NavigationMode) => void;
};

const STORAGE_KEY = 'crunchy-nav-mode';

const NavigationModeContext = createContext<NavigationModeState | undefined>(undefined);

export function NavigationProvider({ children }: { children: ReactNode }) {
  const [mode, setMode] = useState<NavigationMode>(() => {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem(STORAGE_KEY) as NavigationMode | null;
      if (stored === 'sidebar' || stored === 'dock' || stored === 'both') return stored;
    }
    return 'both';
  });

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, mode);
  }, [mode]);

  const value = useMemo(() => ({ mode, setMode }), [mode]);

  return (
    <NavigationModeContext.Provider value={value}>
      {children}
    </NavigationModeContext.Provider>
  );
}

export function useNavigationMode() {
  const context = useContext(NavigationModeContext);
  if (!context) throw new Error('useNavigationMode must be used within a NavigationProvider');
  return context;
}
