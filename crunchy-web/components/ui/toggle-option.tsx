import { Switch } from './switch';
import { cn } from '@/lib/utils';

interface ToggleOptionProps {
  icon: React.ComponentType<{ className?: string }>;
  iconClassName?: string;
  title: string;
  description: string;
  checked: boolean;
  disabled?: boolean;
  recommended?: boolean;
  experimental?: boolean;
  onCheckedChange: (checked: boolean) => void;
  className?: string;
}

export function ToggleOption({
  icon: Icon,
  iconClassName,
  title,
  description,
  checked,
  disabled,
  recommended,
  experimental,
  onCheckedChange,
  className,
}: ToggleOptionProps) {
  return (
    <button
      type='button'
      className={cn(
        'flex w-full items-center justify-between py-4 px-5 text-left bg-secondary/40 transition-colors hover:bg-secondary/70 disabled:opacity-50',
        className
      )}
      disabled={disabled}
      onClick={() => onCheckedChange(!checked)}
    >
      <div className='flex items-center gap-4'>
        <div className='flex size-10 shrink-0 items-center justify-center rounded-lg border bg-muted/50'>
          <Icon className={cn('size-5', iconClassName)} />
        </div>
        <div>
          <p className='text-sm font-medium'>{title} {recommended && <span className='text-xs font-medium text-primary'>Recommended</span>}{experimental && <span className='text-xs font-medium text-yellow-500'>Experimental</span>}</p>
          <p className='text-xs text-muted-foreground'>{description}</p>
        </div>
      </div>
      <Switch
        checked={checked}
        tabIndex={-1}
        disabled={disabled}
      />
    </button>
  );
}
