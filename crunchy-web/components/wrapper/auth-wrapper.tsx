'use client';

import { ReactNode, useEffect } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import { isPublicPath } from '@/lib/auth-paths';
import { SidebarWrapper } from '@/components/wrapper/sidebar-wrapper';
import { WebSocketProvider } from '@/components/providers/websocket-provider';
import { DownloadNotificationProvider } from '@/components/providers/download-notification-provider';
import { LoaderCircleIcon } from 'lucide-react';

export function AuthWrapper({ children }: { children: ReactNode }) {
  const { isAuthenticated, isLoading } = useAuthToken();
  const pathname = usePathname();
  const router = useRouter();

  useEffect(() => {
    if (isLoading) return;

    if (!isAuthenticated && !isPublicPath(pathname)) {
      router.push('/login');
    }

    if (isAuthenticated && isPublicPath(pathname)) {
      router.push('/');
    }
  }, [isAuthenticated, isLoading, pathname, router]);

  if (isLoading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background">
        <LoaderCircleIcon className="size-8 animate-spin text-primary" />
      </div>
    );
  }

  // On a public path (login/sign-up), render without the app shell
  if (isPublicPath(pathname)) {
    if (isAuthenticated) return null; // Will redirect
    return <>{children}</>;
  }

  // Not authenticated and not on public path — will be redirected
  if (!isAuthenticated) {
    return null;
  }

  // Authenticated + protected path: render with sidebar shell
  return (
    <WebSocketProvider>
      <DownloadNotificationProvider>
        <SidebarWrapper>{children}</SidebarWrapper>
      </DownloadNotificationProvider>
    </WebSocketProvider>
  );
}
