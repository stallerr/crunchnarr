'use client';

import * as React from 'react';
import { CheckIcon, ChevronsUpDownIcon, XIcon } from 'lucide-react';

import { cn } from '@/lib/utils';
import { Badge } from '@/components/ui/badge';
import { Popover, PopoverTrigger, PopoverContent } from '@/components/ui/popover';
import { ScrollArea } from '@/components/ui/scroll-area';

interface MultiSelectOption {
  label: string;
  value: string;
}

interface MultiSelectProps {
  options: MultiSelectOption[];
  value: string[];
  onValueChange: (value: string[]) => void;
  placeholder?: string;
  searchPlaceholder?: string;
  maxDisplayed?: number;
  className?: string;
}

function MultiSelect({
  options,
  value,
  onValueChange,
  placeholder = 'Select options',
  searchPlaceholder = 'Search...',
  maxDisplayed = 4,
  className,
}: MultiSelectProps) {
  const [open, setOpen] = React.useState(false);
  const [search, setSearch] = React.useState('');

  const filtered = React.useMemo(() => {
    if (!search) return options;
    const lower = search.toLowerCase();
    return options.filter(
      (opt) =>
        opt.label.toLowerCase().includes(lower) ||
        opt.value.toLowerCase().includes(lower)
    );
  }, [options, search]);

  const toggle = (optionValue: string) => {
    if (value.includes(optionValue)) {
      onValueChange(value.filter((v) => v !== optionValue));
    } else {
      onValueChange([...value, optionValue]);
    }
  };

  const toggleAll = () => {
    if (value.length === options.length) {
      onValueChange([]);
    } else {
      onValueChange(options.map((o) => o.value));
    }
  };

  const clear = () => onValueChange([]);

  const removeTag = (optionValue: string, e: React.MouseEvent) => {
    e.stopPropagation();
    onValueChange(value.filter((v) => v !== optionValue));
  };

  const getLabel = (val: string) =>
    options.find((o) => o.value === val)?.label ?? val;

  React.useEffect(() => {
    if (!open) setSearch('');
  }, [open]);

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        className={cn(
          'flex min-h-9 w-full items-center justify-between rounded-lg border border-input bg-background px-3 py-1.5 text-left text-base shadow-xs/5 outline-none ring-ring/24 transition-shadow focus-visible:border-ring focus-visible:ring-[3px] sm:min-h-8 sm:text-sm dark:bg-input/32',
          className
        )}
      >
        <div className="flex flex-1 flex-wrap items-center gap-1 overflow-hidden">
          {value.length === 0 ? (
            <span className="text-muted-foreground">{placeholder}</span>
          ) : (
            <>
              {value.slice(0, maxDisplayed).map((val) => (
                <Badge
                  key={val}
                  variant="secondary"
                  className="gap-1 pr-0.5"
                >
                  {getLabel(val)}
                  <div
                    role="button"
                    tabIndex={0}
                    className="ml-0.5 rounded-sm p-0.5 hover:bg-foreground/10 cursor-pointer"
                    onClick={(e) => removeTag(val, e)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        removeTag(val, e as unknown as React.MouseEvent);
                      }
                    }}
                    aria-label={`Remove ${getLabel(val)}`}
                  >
                    <XIcon className="size-3" />
                  </div>
                </Badge>
              ))}
              {value.length > maxDisplayed && (
                <Badge variant="outline" className="text-muted-foreground">
                  +{value.length - maxDisplayed} more
                </Badge>
              )}
            </>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-1 ml-2">
          {value.length > 0 && (
            <div
              role="button"
              tabIndex={0}
              className="rounded-sm p-0.5 text-muted-foreground hover:text-foreground cursor-pointer"
              onClick={(e) => {
                e.stopPropagation();
                clear();
              }}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  e.stopPropagation();
                  clear();
                }
              }}
              aria-label="Clear all"
            >
              <XIcon className="size-4" />
            </div>
          )}
          <ChevronsUpDownIcon className="size-4 text-muted-foreground" />
        </div>
      </PopoverTrigger>

      <PopoverContent className="w-(--anchor-width) flex-col p-0">
        <div className="border-b p-2">
          <input
            type="text"
            className="h-8 w-full rounded-md bg-transparent px-2 text-sm outline-none placeholder:text-muted-foreground"
            placeholder={searchPlaceholder}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            autoFocus
          />
        </div>

        <ScrollArea className="max-h-56">
          <div className="p-1">
            {!search && (
              <button
                type="button"
                className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-accent"
                onClick={toggleAll}
              >
                <span
                  className={cn(
                    'flex size-4 items-center justify-center rounded-sm border border-primary',
                    value.length === options.length
                      ? 'bg-primary text-primary-foreground'
                      : 'opacity-50'
                  )}
                >
                  {value.length === options.length && (
                    <CheckIcon className="size-3" />
                  )}
                </span>
                <span>
                  {value.length === options.length ? 'Deselect All' : 'Select All'}
                </span>
              </button>
            )}

            {filtered.length === 0 ? (
              <p className="py-4 text-center text-sm text-muted-foreground">
                No results found.
              </p>
            ) : (
              filtered.map((option) => {
                const selected = value.includes(option.value);
                return (
                  <button
                    key={option.value}
                    type="button"
                    className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-accent"
                    onClick={() => toggle(option.value)}
                  >
                    <span
                      className={cn(
                        'flex size-4 items-center justify-center rounded-sm border border-primary',
                        selected
                          ? 'bg-primary text-primary-foreground'
                          : 'opacity-50'
                      )}
                    >
                      {selected && <CheckIcon className="size-3" />}
                    </span>
                    <span>{option.label}</span>
                  </button>
                );
              })
            )}
          </div>
        </ScrollArea>
      </PopoverContent>
    </Popover>
  );
}

export { MultiSelect };
export type { MultiSelectOption, MultiSelectProps };
