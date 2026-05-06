/* eslint-disable @typescript-eslint/no-explicit-any */

export type SuccessResult<T = any> = {
  success: true;
  status: number;
  data: T;
};

export type ErrorResult<E = any> =
  | {
      success: false;
      status: number;
      data: E;
    }
  | {
      success: false;
      status: null;
      data: null;
    };

export type Result<T = any, E = any> = SuccessResult<T> | ErrorResult<E>;

export type AuthTokens = {
  access_token: string;
  refresh_token: string;
};

export type AuthUser = {
  id: string;
  email: string;
  username: string;
};

export type LoginRequest = {
  email: string;
  password: string;
};

export type RegisterRequest = {
  username: string;
  email: string;
  password: string;
};

export type ApiError = {
  error: string;
  message?: string;
};
