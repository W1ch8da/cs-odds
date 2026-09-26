"use client";

import { ThemeProvider as NextThemesProvider } from "next-themes";

/** Light, dark or follow the OS. Sets `class="dark"` on <html>. */
export function ThemeProvider({ children }: { children: React.ReactNode }) {
  return (
    <NextThemesProvider attribute="class" defaultTheme="system" enableSystem disableTransitionOnChange>
      {children}
    </NextThemesProvider>
  );
}
