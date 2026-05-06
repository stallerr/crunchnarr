import type { Metadata } from 'next';
import { Inter, JetBrains_Mono, Outfit } from 'next/font/google';
import { ThemeProvider } from '@/components/ui/theme-provider';
import { ToastProvider } from '@/components/ui/toast';
import { AuthTokenProvider } from '@/components/providers/auth-token-provider';
import { NavigationProvider } from '@/components/providers/navigation-provider';
import { AccentColorProvider } from '@/components/providers/accent-color-provider';
import { DensityProvider } from '@/components/providers/density-provider';
import { ConfirmCancelProvider } from '@/components/providers/confirm-cancel-provider';
import { AuthWrapper } from '@/components/wrapper/auth-wrapper';
import './globals.css';

const inter = Inter({
  variable: '--font-inter',
  subsets: ['latin'],
});

const outfit = Outfit({
  variable: '--font-outfit',
  subsets: ['latin'],
});

const jetbrainsMono = JetBrains_Mono({
  variable: '--font-jetbrains-mono',
  subsets: ['latin'],
});

export const metadata: Metadata = {
  title: 'Crunchy',
  description: 'Crunchyroll content manager',
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className="dark" suppressHydrationWarning>
      <body
        className={`${inter.variable} ${outfit.variable} ${jetbrainsMono.variable} antialiased`}
      >
        <ThemeProvider defaultTheme="dark" storageKey="crunchy-theme">
          <AccentColorProvider>
            <NavigationProvider>
              <DensityProvider>
                <ConfirmCancelProvider>
                  <ToastProvider>
                    <AuthTokenProvider>
                      <AuthWrapper>
                        {children}
                      </AuthWrapper>
                    </AuthTokenProvider>
                  </ToastProvider>
                </ConfirmCancelProvider>
              </DensityProvider>
            </NavigationProvider>
          </AccentColorProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
