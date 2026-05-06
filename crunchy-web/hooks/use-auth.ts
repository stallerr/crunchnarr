'use client';

import { useState, useCallback } from 'react';
import { useRouter } from 'next/navigation';
import axios from 'axios';
import { useAuthToken } from '@/components/providers/auth-token-provider';
import type { LoginRequest, RegisterRequest } from '@/types/api';

export function useAuth() {
  const router = useRouter();
  const { checkSession } = useAuthToken();
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const loginWithCredentials = useCallback(
    async (credentials: LoginRequest) => {
      setError(null);
      setIsLoading(true);

      try {
        await axios.post('/api/auth/login', credentials);
        await checkSession();
        router.push('/');
      } catch (err) {
        if (axios.isAxiosError(err) && err.response?.data?.error) {
          setError(err.response.data.error);
        } else {
          setError('Login failed. Please try again.');
        }
      } finally {
        setIsLoading(false);
      }
    },
    [checkSession, router]
  );

  const register = useCallback(
    async (data: RegisterRequest) => {
      setError(null);
      setIsLoading(true);

      try {
        await axios.post('/api/auth/register', data);
        await checkSession();
        router.push('/');
      } catch (err) {
        if (axios.isAxiosError(err) && err.response?.data?.error) {
          setError(err.response.data.error);
        } else {
          setError('Registration failed. Please try again.');
        }
      } finally {
        setIsLoading(false);
      }
    },
    [checkSession, router]
  );

  const logout = useCallback(async () => {
    try {
      await axios.post('/api/auth/logout');
    } finally {
      await checkSession();
      router.push('/login');
    }
  }, [checkSession, router]);

  return {
    loginWithCredentials,
    register,
    logout,
    error,
    setError,
    isLoading,
  };
}
