'use client';

import { useCallback, useSyncExternalStore } from 'react';

const STORAGE_KEY = 'crunchy-avatar-id';
const DEFAULT_AVATAR_ID = 1;

const listeners = new Set<() => void>();

function getSnapshot(): number {
  if (typeof window === 'undefined') return DEFAULT_AVATAR_ID;
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored ? Number(stored) : DEFAULT_AVATAR_ID;
}

function getServerSnapshot(): number {
  return DEFAULT_AVATAR_ID;
}

function subscribe(callback: () => void) {
  listeners.add(callback);
  return () => listeners.delete(callback);
}

export function useAvatar() {
  const avatarId = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);

  const setAvatarId = useCallback((id: number) => {
    localStorage.setItem(STORAGE_KEY, String(id));
    listeners.forEach((l) => l());
  }, []);

  return { avatarId, setAvatarId };
}
