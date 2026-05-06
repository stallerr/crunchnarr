'use client';

import { createContext, useContext, useEffect, useState, useMemo, type ReactNode } from 'react';

const STORAGE_KEY = 'crunchy-skip-cancel-confirm';

type ConfirmCancelState = {
  skipConfirm: boolean;
  setSkipConfirm: (v: boolean) => void;
};

const ConfirmCancelContext = createContext<ConfirmCancelState | undefined>(undefined);

export function ConfirmCancelProvider({ children }: { children: ReactNode }) {
  const [skipConfirm, setSkipConfirm] = useState<boolean>(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem(STORAGE_KEY) === 'true';
    }
    return false;
  });

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, String(skipConfirm));
  }, [skipConfirm]);

  const value = useMemo(() => ({ skipConfirm, setSkipConfirm }), [skipConfirm]);

  return (
    <ConfirmCancelContext.Provider value={value}>
      {children}
    </ConfirmCancelContext.Provider>
  );
}

export function useConfirmCancel() {
  const context = useContext(ConfirmCancelContext);
  if (!context) throw new Error('useConfirmCancel must be used within a ConfirmCancelProvider');
  return context;
}
