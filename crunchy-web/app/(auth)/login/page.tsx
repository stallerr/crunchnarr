'use client';

import { useState } from 'react';
import Link from 'next/link';
import { useAuth } from '@/hooks/use-auth';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { PasswordInput } from '@/components/ui/password-input';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel, CardFooter } from '@/components/ui/card';
import { Field, FieldLabel } from '@/components/ui/field';
import { BorderBeam } from '@/components/ui/border-beam';
import {Group, LoaderCircleIcon, PlusIcon} from 'lucide-react';
import {GroupSeparator} from "@/components/ui/group.tsx";
import {Spotlight} from "@/components/ui/spotlight.tsx";

export default function LoginPage() {
  const { loginWithCredentials, error, isLoading } = useAuth();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await loginWithCredentials({ email, password });
  };

  return (
    <Card className="w-full max-w-md overflow-hidden relative">
      <BorderBeam colorFrom="var(--primary)" colorTo="var(--primary)" size={300} duration={4} transition={{ type: "spring", stiffness: 30, damping: 15 }} />
      <CardHeader>
        <CardTitle className="text-2xl font-display">
          <span className="text-primary">Crunchy</span>
        </CardTitle>
        <CardDescription>
          Sign in to your account to continue
        </CardDescription>
      </CardHeader>

      <CardPanel>
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          {error && (
            <div className="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive-foreground">
              {error}
            </div>
          )}

          <Field>
            <FieldLabel>Email</FieldLabel>
            <Input
              type="email"
              name="email"
              placeholder="you@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              autoComplete="email"
              autoFocus
            />
          </Field>

          <Field>
            <FieldLabel>Password</FieldLabel>
            <PasswordInput
              name="password"
              placeholder="Enter your password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              autoComplete="current-password"
            />
          </Field>

          <Button
            type="submit"
            disabled={isLoading}
            className="w-full mt-2"
          >
            {isLoading ? (
              <>
                <LoaderCircleIcon className="animate-spin" />
                Signing in...
              </>
            ) : (
              'Sign in'
            )}
          </Button>
        </form>
      </CardPanel>

      <CardFooter className="justify-center">
        <p className="text-sm text-muted-foreground">
          Don&apos;t have an account?{' '}
          <Link href="/sign-up" className="text-primary hover:underline font-medium">
            Sign up
          </Link>
        </p>
      </CardFooter>
    </Card>
  );
}
