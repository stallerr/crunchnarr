'use client';

import { ReactNode } from 'react';
import { useDownloadNotifications } from '@/hooks/use-download-notifications';

export function DownloadNotificationProvider({ children }: { children: ReactNode }) {
  useDownloadNotifications();
  return <>{children}</>;
}
