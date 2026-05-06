import { cn } from '@/lib/utils';

export function PagePanel({
  className,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn('flex-1 overflow-auto p-6', className)}
      {...props}
    >
      {children}
    </div>
  );
}

export function PageHeader({
  className,
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn('flex flex-col gap-1 mb-6', className)}
      {...props}
    >
      {children}
    </div>
  );
}

export function PageTitle({
  className,
  children,
  ...props
}: React.HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h1
      className={cn('text-2xl font-semibold font-display tracking-tight', className)}
      {...props}
    >
      {children}
    </h1>
  );
}

export function PageDescription({
  className,
  children,
  ...props
}: React.HTMLAttributes<HTMLParagraphElement>) {
  return (
    <p
      className={cn('text-sm text-muted-foreground', className)}
      {...props}
    >
      {children}
    </p>
  );
}
