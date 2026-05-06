'use client';

import { useState } from 'react';
import { CheckCircleIcon, KeyIcon, UserIcon } from 'lucide-react';
import { Card, CardHeader, CardTitle, CardDescription, CardPanel } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { PasswordInput } from '@/components/ui/password-input';
import { Field, FieldLabel } from '@/components/ui/field';
import { useCrunchyrollLogin } from '@/hooks/use-crunchyroll';

type LinkAccountCardProps = {
  onSuccess?: () => void;
};

export function LinkAccountCard({ onSuccess }: LinkAccountCardProps) {
  const [mode, setMode] = useState<'credentials' | 'token'>('credentials');
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [refreshToken, setRefreshToken] = useState('');
  const [linked, setLinked] = useState(false);
  const { login, loginWithToken, error, setError, isLoading } = useCrunchyrollLogin();

  const handleCredentialLogin = async (e: React.SyntheticEvent) => {
    e.preventDefault();
    if (!username || !password) return;
    const success = await login(username, password);
    if (success) {
      setLinked(true);
      onSuccess?.();
    }
  };

  const handleTokenLogin = async (e: React.SyntheticEvent) => {
    e.preventDefault();
    if (!refreshToken) return;
    const success = await loginWithToken(refreshToken);
    if (success) {
      setLinked(true);
      onSuccess?.();
    }
  };

  if (linked) {
    return (
      <Card className="max-w-md mx-auto">
        <CardPanel className="flex flex-col items-center gap-4 py-8">
          <CheckCircleIcon className="size-12 text-green-500" />
          <p className="text-lg font-semibold">Crunchyroll Account Linked</p>
          <p className="text-sm text-muted-foreground">
            Your account has been successfully connected.
          </p>
        </CardPanel>
      </Card>
    );
  }

  return (
    <Card className="max-w-md mx-auto">
      <CardHeader>
        <CardTitle>Link Crunchyroll Account</CardTitle>
        <CardDescription>
          Connect your Crunchyroll account to search and download content.
        </CardDescription>
      </CardHeader>

      <CardPanel>
        <div className="flex gap-2 mb-6">
          <Button
            variant={mode === 'credentials' ? 'default' : 'outline'}
            size="sm"
            onClick={() => { setMode('credentials'); setError(null); }}
          >
            <UserIcon />
            Credentials
          </Button>
          <Button
            variant={mode === 'token' ? 'default' : 'outline'}
            size="sm"
            onClick={() => { setMode('token'); setError(null); }}
          >
            <KeyIcon />
            Refresh Token
          </Button>
        </div>

        {mode === 'credentials' ? (
          <form onSubmit={handleCredentialLogin} className="flex flex-col gap-4">
            <Field>
              <FieldLabel>Email or Username</FieldLabel>
              <Input
                name="username"
                value={username}
                onChange={(e) => setUsername((e.target as HTMLInputElement).value)}
                placeholder="your@email.com"
                autoComplete="username"
              />
            </Field>
            <Field>
              <FieldLabel>Password</FieldLabel>
              <PasswordInput
                name="password"
                value={password}
                onChange={(e) => setPassword((e.target as HTMLInputElement).value)}
                placeholder="Your Crunchyroll password"
                autoComplete="current-password"
              />
            </Field>
            {error && <p className="text-destructive-foreground text-xs">{error}</p>}
            <Button type="submit" disabled={isLoading || !username || !password}>
              {isLoading ? 'Connecting...' : 'Link Account'}
            </Button>
          </form>
        ) : (
          <form onSubmit={handleTokenLogin} className="flex flex-col gap-4">
            <Field>
              <FieldLabel>Refresh Token</FieldLabel>
              <Input
                value={refreshToken}
                onChange={(e) => setRefreshToken((e.target as HTMLInputElement).value)}
                placeholder="Paste your Crunchyroll refresh token"
                autoComplete="off"
              />
            </Field>
            <p className="text-xs text-muted-foreground">
              Advanced: Use a refresh token from a previous Crunchyroll session.
            </p>
            {error && <p className="text-destructive-foreground text-xs">{error}</p>}
            <Button type="submit" disabled={isLoading || !refreshToken}>
              {isLoading ? 'Connecting...' : 'Link with Token'}
            </Button>
          </form>
        )}
      </CardPanel>
    </Card>
  );
}
