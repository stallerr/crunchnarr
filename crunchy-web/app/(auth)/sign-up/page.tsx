'use client';

import { useState } from 'react';
import Link from 'next/link';
import { useAuth } from '@/hooks/use-auth';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { PasswordInput } from '@/components/ui/password-input';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel, CardFooter } from '@/components/ui/card';
import { Field, FieldLabel, FieldError } from '@/components/ui/field';
import { BorderBeam } from '@/components/ui/border-beam';
import { LoaderCircleIcon } from 'lucide-react';

export default function SignUpPage() {
  const { register, error, isLoading } = useAuth();
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [validationError, setValidationError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setValidationError(null);

    if (password !== confirmPassword) {
      setValidationError('Passwords do not match');
      return;
    }

    if (password.length < 8) {
      setValidationError('Password must be at least 8 characters');
      return;
    }

    await register({ username, email, password });
  };

  const displayError = validationError || error;

  return (
    <Card className="w-full max-w-md overflow-hidden">
      <BorderBeam colorFrom="var(--primary)" colorTo="var(--primary)" size={300} duration={4} transition={{ type: "spring", stiffness: 30, damping: 15 }} />
      <CardHeader>
        <CardTitle className="text-2xl font-display">
          <span className="text-primary">Create account</span>
        </CardTitle>
        <CardDescription>
          Get started with Crunchy
        </CardDescription>
      </CardHeader>

      <CardPanel>
        <form onSubmit={handleSubmit} className="flex flex-col gap-4">
          {displayError && (
            <div className="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3 text-sm text-destructive-foreground">
              {displayError}
            </div>
          )}

          <Field>
            <FieldLabel>Username</FieldLabel>
            <Input
              type="text"
              name="username"
              placeholder="Choose a username"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
              autoComplete="username"
              autoFocus
            />
          </Field>

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
            />
          </Field>

          <Field>
            <FieldLabel>Password</FieldLabel>
            <PasswordInput
              name="password"
              placeholder="Create a password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              required
              autoComplete="new-password"
            />
            {password.length > 0 && password.length < 8 && (
              <FieldError>Must be at least 8 characters</FieldError>
            )}
          </Field>

          <Field>
            <FieldLabel>Confirm password</FieldLabel>
            <PasswordInput
              name="password-confirm"
              placeholder="Confirm your password"
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              required
              autoComplete="new-password"
            />
            {confirmPassword.length > 0 && password !== confirmPassword && (
              <FieldError>Passwords do not match</FieldError>
            )}
          </Field>

          <Button
            type="submit"
            disabled={isLoading}
            className="w-full mt-2"
          >
            {isLoading ? (
              <>
                <LoaderCircleIcon className="animate-spin" />
                Creating account...
              </>
            ) : (
              'Create account'
            )}
          </Button>
        </form>
      </CardPanel>

      <CardFooter className="justify-center">
        <p className="text-sm text-muted-foreground">
          Already have an account?{' '}
          <Link href="/login" className="text-primary hover:underline font-medium">
            Sign in
          </Link>
        </p>
      </CardFooter>
    </Card>
  );
}
