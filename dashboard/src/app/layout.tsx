import "./globals.css";
import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import type { ReactNode } from "react";

const mono = JetBrains_Mono({ subsets: ["latin"], variable: "--font-mono" });
const sans = Inter({ subsets: ["latin"], variable: "--font-sans" });

export const metadata: Metadata = {
  title: "Rune",
  description: "Rust autonomous agent framework dashboard",
};

/**
 * Root HTML layout. Loads JetBrains Mono + Geist via `next/font` and
 * forces a dark cyberpunk theme on every page.
 */
export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={`${mono.variable} ${sans.variable}`}>
      <body className="bg-bg text-primary">{children}</body>
    </html>
  );
}
