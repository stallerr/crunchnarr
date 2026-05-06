'use client';

import Link from 'next/link';
import { LinkIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function LinkBanner() {
  return (
    <div className="rounded-xl border border-primary/30 bg-primary/5 p-6 flex items-center gap-4">
      <div className="rounded-full bg-primary/10 p-3">
        <LinkIcon className="size-6 text-primary" />
      </div>
      <div className="flex-1">
        <h3 className="font-semibold text-lg">Link your Crunchyroll account</h3>
        <p className="text-sm text-muted-foreground">
          Connect your account to search, browse, and download anime.
        </p>
      </div>
      <Button render={<Link href="/link-crunchyroll" />}>
        Get Started
      </Button>
    </div>
  );
}
