'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import {
  BookmarkIcon,
  ClapperboardIcon,
  DownloadIcon,
  HardDriveIcon, HouseIcon,
  LayoutDashboardIcon,
  SearchIcon,
  SettingsIcon,
  UserIcon,
} from 'lucide-react';
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from '@/components/ui/sidebar';

export type NavItem = {
  label: string;
  href: string;
  icon: React.ReactNode;
  exact?: boolean;
};

export const mainNavItems: NavItem[] = [
  { label: 'Home', href: '/', icon: <HouseIcon />, exact: true },
  { label: 'Search', href: '/search', icon: <SearchIcon /> },
  { label: 'Bookmarks', href: '/bookmarks', icon: <BookmarkIcon /> },
  { label: 'Watchlist', href: '/watchlist', icon: <ClapperboardIcon /> },
  { label: 'Downloads', href: '/downloads', icon: <DownloadIcon /> },
];

export const settingsNavItems: NavItem[] = [
  { label: 'Account', href: '/account', icon: <UserIcon /> },
  { label: 'Settings', href: '/settings', icon: <SettingsIcon /> },
  // { label: 'Cache', href: '/cache', icon: <HardDriveIcon /> },
];

export function SidebarNav() {
  const pathname = usePathname();

  const isActive = (item: NavItem) => {
    if (item.exact) return pathname === item.href;
    return pathname.startsWith(item.href);
  };

  return (
    <>
      <SidebarGroup>
        <SidebarGroupContent>
          <SidebarMenu>
            {mainNavItems.map((item) => (
              <SidebarMenuItem key={item.href}>
                <SidebarMenuButton asChild isActive={isActive(item)}>
                  <Link href={item.href}>
                    {item.icon}
                    <span>{item.label}</span>
                  </Link>
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>

      <SidebarGroup>
        <SidebarGroupLabel>Configuration</SidebarGroupLabel>
        <SidebarGroupContent>
          <SidebarMenu>
            {settingsNavItems.map((item) => (
              <SidebarMenuItem key={item.href}>
                <SidebarMenuButton asChild isActive={isActive(item)}>
                  <Link href={item.href}>
                    {item.icon}
                    <span>{item.label}</span>
                  </Link>
                </SidebarMenuButton>
              </SidebarMenuItem>
            ))}
          </SidebarMenu>
        </SidebarGroupContent>
      </SidebarGroup>
    </>
  );
}
