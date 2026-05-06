'use client';

import { createContext, useContext, useEffect, useState, useMemo, type ReactNode } from 'react';

export type Density = 'compact' | 'comfortable';

const STORAGE_KEY = 'crunchy-density';

type DensityState = {
  density: Density;
  setDensity: (d: Density) => void;
};

const DensityContext = createContext<DensityState | undefined>(undefined);

export function DensityProvider({ children }: { children: ReactNode }) {
  const [density, setDensity] = useState<Density>(() => {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem(STORAGE_KEY);
      if (stored === 'compact' || stored === 'comfortable') return stored;
    }
    return 'comfortable';
  });

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, density);
  }, [density]);

  const value = useMemo(() => ({ density, setDensity }), [density]);

  return (
    <DensityContext.Provider value={value}>
      {children}
    </DensityContext.Provider>
  );
}

export function useDensity() {
  const context = useContext(DensityContext);
  if (!context) throw new Error('useDensity must be used within a DensityProvider');
  return context;
}
