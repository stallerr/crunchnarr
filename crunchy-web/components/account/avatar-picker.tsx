'use client';

import type { Variants } from 'motion/react';
import { AnimatePresence, motion, useReducedMotion } from 'motion/react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import { avatars, getAvatarById } from '@/lib/avatars';
import { useAvatar } from '@/hooks/use-avatar';

const containerVariants: Variants = {
  initial: { opacity: 0 },
  animate: {
    opacity: 1,
    transition: { staggerChildren: 0.06, delayChildren: 0.05 },
  },
};

const thumbnailVariants: Variants = {
  initial: { opacity: 0, y: 6 },
  animate: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.28, ease: 'easeOut' },
  },
};

export function AvatarPicker() {
  const { avatarId, setAvatarId } = useAvatar();
  const selectedAvatar = getAvatarById(avatarId);
  const shouldReduceMotion = useReducedMotion();

  const handleSelect = (id: number) => {
    if (id === avatarId) return;
    setAvatarId(id);
  };

  const rgb = selectedAvatar.rgb;

  return (
    <Card>
      <CardHeader>
        <CardTitle>Avatar</CardTitle>
        <CardDescription>Pick an avatar for your profile.</CardDescription>
      </CardHeader>
      <CardPanel>
        <div className="flex flex-col items-center gap-4">
          {/* Stage — large preview */}
          <div className="relative h-32 w-32">
            <motion.div
              animate={{
                boxShadow: `0 0 0 2px rgba(${rgb}, 0.55), 0 6px 24px rgba(${rgb}, 0.18)`,
              }}
              aria-hidden="true"
              className="pointer-events-none absolute inset-0 rounded-full"
              transition={
                shouldReduceMotion ? { duration: 0 } : { duration: 0.45, ease: 'easeOut' }
              }
            />
            <div className="relative h-full w-full overflow-hidden rounded-full">
              <AnimatePresence mode="wait">
                <motion.div
                  key={selectedAvatar.id}
                  animate={{ opacity: 1 }}
                  className="absolute inset-0 flex items-center justify-center"
                  exit={{ opacity: 0 }}
                  initial={{ opacity: 0 }}
                  transition={
                    shouldReduceMotion ? { duration: 0 } : { duration: 0.2, ease: 'easeOut' }
                  }
                >
                  <div className="scale-[3.2] transform">{selectedAvatar.svg}</div>
                </motion.div>
              </AnimatePresence>
            </div>
          </div>

          {/* Name label */}
          <AnimatePresence mode="wait">
            <motion.span
              animate={{ opacity: 1 }}
              className="text-[11px] tracking-[0.12em] text-muted-foreground uppercase"
              exit={{ opacity: 0 }}
              initial={{ opacity: 0 }}
              key={selectedAvatar.id}
              transition={
                shouldReduceMotion ? { duration: 0 } : { duration: 0.16, ease: 'easeOut' }
              }
            >
              {selectedAvatar.name}
            </motion.span>
          </AnimatePresence>

          {/* Thumbnail strip */}
          <motion.div
            animate="animate"
            className="flex gap-3"
            initial="initial"
            variants={containerVariants}
          >
            {avatars.map((avatar) => {
              const isSelected = avatarId === avatar.id;
              return (
                <motion.button
                  aria-label={`Select ${avatar.name}`}
                  aria-pressed={isSelected}
                  className={cn(
                    'relative h-14 w-14 overflow-hidden rounded-xl border bg-muted transition-[opacity,box-shadow] duration-200 ease-out',
                    isSelected
                      ? 'border-foreground/20 opacity-100 ring-2 ring-foreground/70 ring-offset-2 ring-offset-background'
                      : 'border-border opacity-50 hover:opacity-100'
                  )}
                  key={avatar.id}
                  onClick={() => handleSelect(avatar.id)}
                  type="button"
                  variants={thumbnailVariants}
                  whileHover={shouldReduceMotion ? {} : { scale: 1.06 }}
                  whileTap={shouldReduceMotion ? {} : { scale: 0.94 }}
                >
                  <div className="absolute inset-0 flex items-center justify-center">
                    <div className="scale-[2.3] transform">{avatar.svg}</div>
                  </div>
                </motion.button>
              );
            })}
          </motion.div>
        </div>
      </CardPanel>
    </Card>
  );
}
