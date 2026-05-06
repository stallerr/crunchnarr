'use client';

import { Dialog as BaseDialog } from '@base-ui/react/dialog';
import { XIcon } from 'lucide-react';
import { cn } from '@/lib/utils';

function Dialog({ children, ...props }: BaseDialog.Root.Props) {
  return <BaseDialog.Root {...props}>{children}</BaseDialog.Root>;
}

function DialogTrigger({
  className,
  children,
  ...props
}: BaseDialog.Trigger.Props & { className?: string }) {
  return (
    <BaseDialog.Trigger className={className} {...props}>
      {children}
    </BaseDialog.Trigger>
  );
}

function DialogPortal({ children }: { children: React.ReactNode }) {
  return <BaseDialog.Portal>{children}</BaseDialog.Portal>;
}

function DialogBackdrop({ className }: { className?: string }) {
  return (
    <BaseDialog.Backdrop
      className={cn(
        'fixed inset-0 z-50 bg-black/50 backdrop-blur-sm',
        'transition-all duration-150',
        'data-[starting-style]:opacity-0 data-[ending-style]:opacity-0',
        className
      )}
    />
  );
}

function DialogPopup({
  className,
  children,
  ...props
}: BaseDialog.Popup.Props & { className?: string }) {
  return (
    <BaseDialog.Popup
      className={cn(
        'fixed top-1/2 left-1/2 z-50 w-full max-w-lg -translate-x-1/2 -translate-y-1/2',
        'rounded-xl border bg-card p-6 shadow-lg',
        'transition-all duration-150',
        'data-[starting-style]:scale-95 data-[starting-style]:opacity-0',
        'data-[ending-style]:scale-95 data-[ending-style]:opacity-0',
        className
      )}
      {...props}
    >
      {children}
    </BaseDialog.Popup>
  );
}

function DialogTitle({
  className,
  children,
  ...props
}: BaseDialog.Title.Props & { className?: string }) {
  return (
    <BaseDialog.Title
      className={cn('text-lg font-semibold', className)}
      {...props}
    >
      {children}
    </BaseDialog.Title>
  );
}

function DialogDescription({
  className,
  children,
  ...props
}: BaseDialog.Description.Props & { className?: string }) {
  return (
    <BaseDialog.Description
      className={cn('text-sm text-muted-foreground', className)}
      {...props}
    >
      {children}
    </BaseDialog.Description>
  );
}

function DialogClose({
  className,
  children,
  ...props
}: BaseDialog.Close.Props & { className?: string }) {
  return (
    <BaseDialog.Close className={className} {...props}>
      {children}
    </BaseDialog.Close>
  );
}

function DialogCloseX({ className }: { className?: string }) {
  return (
    <BaseDialog.Close
      className={cn(
        'absolute right-4 top-4 rounded-sm opacity-70 transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring',
        className
      )}
    >
      <XIcon className="size-4" />
      <span className="sr-only">Close</span>
    </BaseDialog.Close>
  );
}

export {
  Dialog,
  DialogTrigger,
  DialogPortal,
  DialogBackdrop,
  DialogPopup,
  DialogTitle,
  DialogDescription,
  DialogClose,
  DialogCloseX,
};
