import { NextRequest, NextResponse } from 'next/server';
import axios from 'axios';

const API_URL = process.env.API_URL ?? process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:8080';

export async function POST(request: NextRequest) {
  const refreshToken = request.cookies.get('refresh_token')?.value;

  if (!refreshToken) {
    return NextResponse.json(
      { error: 'No refresh token' },
      { status: 401 }
    );
  }

  try {
    const response = await axios.post(
      `${API_URL}/auth/refresh`,
      { refresh_token: refreshToken },
      { timeout: 15_000 }
    );

    const { access_token, refresh_token: newRefreshToken } = response.data;

    const res = NextResponse.json({ success: true }, { status: 200 });

    res.cookies.set('access_token', access_token, {
      httpOnly: true,
      secure: process.env.NODE_ENV === 'production',
      sameSite: 'lax',
      path: '/',
      maxAge: 60 * 60,
    });

    if (newRefreshToken) {
      res.cookies.set('refresh_token', newRefreshToken, {
        httpOnly: true,
        secure: process.env.NODE_ENV === 'production',
        sameSite: 'lax',
        path: '/',
        maxAge: 60 * 60 * 24 * 7,
      });
    }

    return res;
  } catch (error) {
    if (axios.isAxiosError(error) && error.response) {
      // If refresh fails, clear cookies
      const res = NextResponse.json(
        { error: 'Token refresh failed' },
        { status: 401 }
      );
      res.cookies.set('access_token', '', { path: '/', maxAge: 0 });
      res.cookies.set('refresh_token', '', { path: '/', maxAge: 0 });
      return res;
    }
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}
