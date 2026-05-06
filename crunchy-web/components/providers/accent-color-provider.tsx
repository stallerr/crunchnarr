'use client';

import { createContext, useContext, useEffect, useState, useMemo, type ReactNode } from 'react';

export type AccentColor = {
  name: string;
  value: string; // oklch value for --primary
};

export const ACCENT_COLORS: AccentColor[] = [
  { name: 'Orange',  value: 'oklch(0.72 0.17 52)' },
  { name: 'Red',     value: 'oklch(0.63 0.24 25)' },
  { name: 'Pink',    value: 'oklch(0.65 0.24 350)' },
  { name: 'Purple',  value: 'oklch(0.60 0.24 300)' },
  { name: 'Blue',    value: 'oklch(0.62 0.20 260)' },
  { name: 'Cyan',    value: 'oklch(0.72 0.14 200)' },
  { name: 'Green',   value: 'oklch(0.68 0.19 150)' },
  { name: 'Yellow',  value: 'oklch(0.80 0.18 85)' },
];

const DEFAULT_COLOR = ACCENT_COLORS[0].value;
const STORAGE_KEY = 'crunchy-accent-color';

type AccentColorState = {
  accentColor: string;
  setAccentColor: (value: string) => void;
};

const AccentColorContext = createContext<AccentColorState | undefined>(undefined);

export function AccentColorProvider({ children }: { children: ReactNode }) {
  const [accentColor, setAccentColor] = useState<string>(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem(STORAGE_KEY) || DEFAULT_COLOR;
    }
    return DEFAULT_COLOR;
  });

  useEffect(() => {
    document.documentElement.style.setProperty('--primary', accentColor);
    localStorage.setItem(STORAGE_KEY, accentColor);
  }, [accentColor]);

  const value = useMemo(() => ({ accentColor, setAccentColor }), [accentColor]);

  return (
    <AccentColorContext.Provider value={value}>
      {children}
    </AccentColorContext.Provider>
  );
}

export function useAccentColor() {
  const context = useContext(AccentColorContext);
  if (!context) throw new Error('useAccentColor must be used within an AccentColorProvider');
  return context;
}
