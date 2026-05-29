import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: ["class", '[data-theme="dark"]'],
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    container: {
      center: true,
      padding: "2rem",
      screens: {
        "2xl": "1400px",
      },
    },
    extend: {
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
        sidebar: {
          DEFAULT: "hsl(var(--sidebar))",
          foreground: "hsl(var(--sidebar-foreground))",
        },
      },
      borderRadius: {
        xl: "calc(var(--radius) + 4px)",
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      fontFamily: {
        // SF Pro Text first so macOS users get the system face automatically.
        // Inter remains as the bundled fallback for non-Apple builds and
        // anywhere the system stack misses (e.g. embedded WebViews on Linux).
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro Text",
          "Inter",
          "system-ui",
          "sans-serif",
        ],
        // Display face for large headings (SF Pro Display variant on macOS).
        display: [
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro Display",
          "Inter",
          "system-ui",
          "sans-serif",
        ],
        serif: ["Spectral", "Georgia", "ui-serif", "serif"],
        // Brand wordmark only ("attune" in the sidebar). Every other
        // heading uses the configured system font; see the `.font-serif`
        // neutraliser in globals.css.
        wordmark: ["Spectral", "Georgia", "ui-serif", "serif"],
        mono: ["JetBrains Mono", "ui-monospace", "monospace"],
      },
      fontSize: {
        // Modular scale from v2 roadmap finding 017. Defaults stay at the
        // Tailwind 12/14/16/18/... grid to avoid mass layout shifts. These
        // named ms-* steps add the Apple HIG 11/13/15/17/22/28/34/45 scale
        // as opt-in tokens.
        "2xs": ["0.6875rem", { lineHeight: "1rem" }], // 11 — caption
        "ms-13": ["0.8125rem", { lineHeight: "1.125rem" }],
        "ms-15": ["0.9375rem", { lineHeight: "1.375rem" }],
        "ms-17": ["1.0625rem", { lineHeight: "1.5rem" }],
        "ms-22": ["1.375rem", { lineHeight: "1.75rem" }],
        "ms-28": ["1.75rem", { lineHeight: "2.125rem" }],
        "ms-34": ["2.125rem", { lineHeight: "2.5rem" }],
        "ms-45": ["2.8125rem", { lineHeight: "3.25rem" }],
      },
      keyframes: {
        "accordion-down": {
          from: { height: "0" },
          to: { height: "var(--radix-accordion-content-height)" },
        },
        "accordion-up": {
          from: { height: "var(--radix-accordion-content-height)" },
          to: { height: "0" },
        },
        "pulse-record": {
          "0%, 100%": { opacity: "1" },
          "50%": { opacity: "0.5" },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
        "pulse-record": "pulse-record 1.4s ease-in-out infinite",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};

export default config;
