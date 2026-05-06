'use client';

import { ScrollArea as ScrollAreaPrimitive } from '@base-ui/react/scroll-area';

import { cn } from '@/lib/utils';

function ScrollArea({
  className,
  children,
  orientation = 'vertical',
  ...props
}: ScrollAreaPrimitive.Root.Props & {
  orientation?: 'vertical' | 'horizontal' | 'both';
}) {
  return (
    <ScrollAreaPrimitive.Root
      className={cn('relative w-full', className)}
      {...props}
    >
      <ScrollAreaPrimitive.Viewport
        className={cn(
          'max-h-[inherit] rounded-[inherit] outline-none overflow-y-auto',
          orientation === 'horizontal' && 'overflow-x-auto overflow-y-hidden',
          orientation === 'both' && 'overflow-auto',
        )}
        data-slot='scroll-area-viewport'
      >
        {children}
      </ScrollAreaPrimitive.Viewport>
      {(orientation === 'vertical' || orientation === 'both') && <ScrollBar orientation='vertical' />}
      {(orientation === 'horizontal' || orientation === 'both') && (
        <ScrollBar orientation='horizontal' />
      )}
      <ScrollAreaPrimitive.Corner data-slot='scroll-area-corner' />
    </ScrollAreaPrimitive.Root>
  );
}

function ScrollBar({
  className,
  orientation = 'vertical',
  ...props
}: ScrollAreaPrimitive.Scrollbar.Props) {
  return (
    <ScrollAreaPrimitive.Scrollbar
      className={cn(
        'm-1 flex opacity-0 transition-opacity delay-300 data-[orientation=horizontal]:h-1.5 data-[orientation=vertical]:w-1.5 data-[orientation=horizontal]:flex-col data-hovering:opacity-100 data-scrolling:opacity-100 data-hovering:delay-0 data-scrolling:delay-0 data-hovering:duration-100 data-scrolling:duration-100',
        className
      )}
      data-slot='scroll-area-scrollbar'
      orientation={orientation}
      {...props}
    >
      <ScrollAreaPrimitive.Thumb
        className='relative flex-1 rounded-full bg-foreground/20'
        data-slot='scroll-area-thumb'
      />
    </ScrollAreaPrimitive.Scrollbar>
  );
}

export { ScrollArea, ScrollBar };
