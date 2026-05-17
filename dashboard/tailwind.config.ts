import type { Config } from "tailwindcss";

/**
 * Rune dashboard Tailwind configuration. Implements the cyberpunk palette
 * defined in section 6.1 of the project spec.
 */
const config: Config = {
  content: ["./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        bg: "var(--rune-bg)",
        surface: "var(--rune-surface)",
        border: "var(--rune-border)",
        primary: "var(--rune-text-primary)",
        muted: "var(--rune-text-muted)",
        accent: {
          green: "var(--rune-accent-green)",
          red: "var(--rune-accent-red)",
          amber: "var(--rune-accent-amber)",
          cyan: "var(--rune-accent-cyan)",
        },
      },
      fontFamily: {
        mono: ["var(--font-mono)", "ui-monospace", "monospace"],
        sans: ["var(--font-sans)", "ui-sans-serif", "system-ui"],
      },
      boxShadow: {
        panel: "0 0 0 1px var(--rune-border)",
        glow: "0 0 12px rgba(0, 255, 136, 0.18)",
      },
      borderRadius: {
        DEFAULT: "4px",
        sm: "2px",
        md: "4px",
      },
    },
  },
  plugins: [],
};

export default config;
