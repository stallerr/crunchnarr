'use client';

import {ReactNode} from 'react';
import {
    Sidebar,
    SidebarContent,
    SidebarFooter,
    SidebarHeader,
    SidebarProvider,
} from '@/components/ui/sidebar';
import {mainNavItems, NavItem, settingsNavItems, SidebarNav} from '@/components/layout/sidebar-content';
import {ConnectionStatus} from '@/components/layout/connection-status';
import {Header} from '@/components/layout/header';
import {Dock, DockIcon} from "@/components/ui/dock.tsx";
import Link from "next/link";
import {buttonVariants} from "@/components/ui/button.tsx";
import {cn} from "@/lib/utils.ts";
import {Separator} from "@/components/ui/separator.tsx";
import {Tooltip, TooltipContent, TooltipProvider, TooltipTrigger} from "@/components/ui/tooltip.tsx";
import {usePathname} from "next/navigation";
import {useNavigationMode} from "@/components/providers/navigation-provider";
import {ScrollArea} from "@/components/ui/scroll-area.tsx";

export function SidebarWrapper({children}: { children: ReactNode }) {
    const pathname = usePathname();
    const {mode} = useNavigationMode();

    const isActive = (item: NavItem) => {
        if (item.exact) return pathname === item.href;
        return pathname.startsWith(item.href);
    };

    const showSidebar = mode === 'sidebar' || mode === 'both';
    const showDock = mode === 'dock' || mode === 'both';

    return (
        <SidebarProvider>
            {showSidebar && (
                <Sidebar>
                    <SidebarHeader className="border-sidebar-border h-12">
                        <div className="flex items-center gap-2 px-2 py-1">
                            <span className="text-lg font-bold font-display text-primary">Crunchy</span>
                        </div>
                    </SidebarHeader>
                    <SidebarContent className="scroll-bar border-t">
                        <SidebarNav />
                    </SidebarContent>
                    <SidebarFooter className="border-t border-sidebar-border">
                        <ConnectionStatus />
                    </SidebarFooter>
                </Sidebar>
            )}
            <ScrollArea render={<main className="relative max-h-screen flex-1 flex flex-col bg-background" />}>
                <Header/>
                {children}
            </ScrollArea>
            {showDock && (
                <TooltipProvider delay={10}>
                    <Dock iconDistance={100} direction="middle"
                          className="absolute bottom-4 left-1/2 -translate-x-1/2 rounded-full bg-foreground/10 backdrop-blur-sm border border-sidebar-border p-1">
                        {mainNavItems.map((item) => (
                            <DockIcon key={item.href}>
                                <Tooltip>
                                    <TooltipTrigger
                                        render={
                                            <Link
                                                href={item.href}
                                                aria-label={item.label}
                                                className={cn(
                                                    buttonVariants({variant: "ghost", size: "icon-xl"}),
                                                    "size-14 rounded-full [&_svg]:size-4",
                                                    isActive(item) && "bg-primary/10 text-primary"
                                                )}
                                            />
                                        }>
                                        {item.icon}
                                    </TooltipTrigger>
                                    <TooltipContent className="rounded-full text-lg font-semibold py-1 px-2 bg-foreground/5 backdrop-blur-md text-foreground">
                                        {item.label}
                                    </TooltipContent>
                                </Tooltip>
                            </DockIcon>
                        ))}
                        <Separator orientation="vertical" className="h-full"/>
                        {settingsNavItems.map((item) => (
                            <DockIcon key={item.href}>
                                <Tooltip>
                                    <TooltipTrigger
                                        render={
                                            <Link
                                                href={item.href}
                                                aria-label={item.label}
                                                className={cn(
                                                    buttonVariants({variant: "ghost", size: "icon-xl"}),
                                                    "size-14 rounded-full [&_svg]:size-4",
                                                    isActive(item) && "bg-primary/10 text-primary"
                                                )}
                                            />
                                        }>
                                        {item.icon}
                                    </TooltipTrigger>
                                    <TooltipContent className="rounded-full text-lg font-semibold py-1 px-2 bg-foreground/5 backdrop-blur-md text-foreground">
                                        {item.label}
                                    </TooltipContent>
                                </Tooltip>
                            </DockIcon>
                        ))}
                    </Dock>
                </TooltipProvider>
            )}
        </SidebarProvider>
    );
}
