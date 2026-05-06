import axios, { AxiosError, AxiosRequestConfig, AxiosResponse } from 'axios';
import type { CreateAxiosDefaults } from 'axios';
import type { Result } from '@/types/api';

/* eslint-disable @typescript-eslint/no-explicit-any */

const url = (typeof window === 'undefined' ? process.env.API_URL : undefined) ?? process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:8080';
export const BASE_URL = url.endsWith('/') ? url.substring(0, url.length - 1) : url;

const buildSuccessResult = <T = any>(response: AxiosResponse<T>): Result<T> => ({
  success: true,
  status: response.status,
  data: response.data,
});

const buildErrorResult = <E = any>(error: Error | AxiosError<E>): Result<never, E> => {
  if (axios.isAxiosError(error) && error.response?.status) {
    return {
      success: false,
      status: error.response.status,
      data: error.response.data,
    };
  }
  return {
    success: false,
    status: null,
    data: null,
  };
};

export const createClient = (token: string, options: CreateAxiosDefaults = {}) =>
  axios.create({
    baseURL: BASE_URL,
    timeout: 15_000,
    headers: { Authorization: `Bearer ${token}` },
    ...options,
  });

export const createUnauthenticatedClient = (options: CreateAxiosDefaults = {}) =>
  axios.create({
    baseURL: BASE_URL,
    timeout: 15_000,
    ...options,
  });

export const unwrap = async <T = any>(call: Promise<Result<T>> | Result<T>) => {
  const response = await call;
  if (response.success) {
    return { data: response.data, error: null, response };
  }
  const errorMessage =
    response.data && typeof response.data === 'object' && 'error' in response.data
      ? (response.data as { error: string }).error
      : 'Something went wrong';
  return { data: null, error: errorMessage, response };
};

export const get = async <T = any, E = any>(
  token: string,
  endpoint: string,
  config?: AxiosRequestConfig
): Promise<Result<T, E>> =>
  createClient(token)
    .get<T>(endpoint, config)
    .then(buildSuccessResult<T>)
    .catch(buildErrorResult<E>);

export const post = async <T = any, E = any, D = any>(
  token: string,
  endpoint: string,
  data?: D,
  config?: AxiosRequestConfig<D>
): Promise<Result<T, E>> =>
  createClient(token)
    .post<T>(endpoint, data, config)
    .then(buildSuccessResult<T>)
    .catch(buildErrorResult<E>);

export const put = async <T = any, E = any, D = any>(
  token: string,
  endpoint: string,
  data?: D,
  config?: AxiosRequestConfig<D>
): Promise<Result<T, E>> =>
  createClient(token)
    .put<T>(endpoint, data, config)
    .then(buildSuccessResult<T>)
    .catch(buildErrorResult<E>);

export const patch = async <T = any, E = any, D = any>(
  token: string,
  endpoint: string,
  data?: D,
  config?: AxiosRequestConfig<D>
): Promise<Result<T, E>> =>
  createClient(token)
    .patch<T>(endpoint, data, config)
    .then(buildSuccessResult<T>)
    .catch(buildErrorResult<E>);

export const del = async <T = any, E = any>(
  token: string,
  endpoint: string,
  config?: AxiosRequestConfig
): Promise<Result<T, E>> =>
  createClient(token)
    .delete<T>(endpoint, config)
    .then(buildSuccessResult<T>)
    .catch(buildErrorResult<E>);
