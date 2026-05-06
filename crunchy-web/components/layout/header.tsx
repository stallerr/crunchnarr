'use client';

import { SidebarTrigger } from '@/components/ui/sidebar';
import { Button } from '@/components/ui/button';
import { LogOutIcon } from 'lucide-react';
import { useAuth } from '@/hooks/use-auth';
import { useNavigationMode } from '@/components/providers/navigation-provider';

export function Header() {
  const { logout } = useAuth();
  const { mode } = useNavigationMode();

  return (
    <div className="sticky top-0 backdrop-blur-sm supports-backdrop-filter:bg-card/65 z-48 border-b">
      <div className="h-12 w-full flex items-center gap-4 px-4">
        {mode !== 'dock' ? (
          <div className="flex items-center gap-2">
            <SidebarTrigger />
          </div>
        ) : (

        <span className="text-lg font-bold font-display text-primary">Crunchy</span>
        )}

        <div className="flex-1" />

        <Button
          variant="ghost"
          size="sm"
          onClick={logout}
          className="text-muted-foreground hover:text-foreground"
        >
          <LogOutIcon />
          <span className="hidden sm:inline">Sign out</span>
        </Button>
      </div>
    </div>
  );
}
