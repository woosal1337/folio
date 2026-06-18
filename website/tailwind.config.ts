import type { Config } from "tailwindcss";

const config: Config = {
  darkMode: ["class", '[data-theme="dark"]'],
  content: [
    "./app/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./lib/**/*.{ts,tsx}",
  ],
  theme: {
    container: {
      center: true,
      padding: "1.5rem",
      screens: {
        "2xl": "1200px",
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
        signal: {
          DEFAULT: "hsl(var(--signal))",
          foreground: "hsl(var(--signal-foreground))",
        },
      },
      borderRadius: {
        "2xl": "calc(var(--radius) + 8px)",
        xl: "calc(var(--radius) + 4px)",
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      fontFamily: {
        sans: ["var(--font-sans)", "-apple-system", "BlinkMacSystemFont", "system-ui", "sans-serif"],
        display: ["var(--font-display)", "-apple-system", "BlinkMacSystemFont", "system-ui", "sans-serif"],
        serif: ["var(--font-serif)", "Spectral", "Georgia", "ui-serif", "serif"],
        wordmark: ["var(--font-serif)", "Spectral", "Georgia", "ui-serif", "serif"],
        mono: ["var(--font-mono)", "JetBrains Mono", "ui-monospace", "monospace"],
      },
      fontSize: {
        "2xs": ["0.6875rem", { lineHeight: "1rem" }],
        "ms-13": ["0.8125rem", { lineHeight: "1.125rem" }],
        "ms-15": ["0.9375rem", { lineHeight: "1.375rem" }],
        "ms-17": ["1.0625rem", { lineHeight: "1.5rem" }],
        "ms-22": ["1.375rem", { lineHeight: "1.75rem" }],
        "ms-28": ["1.75rem", { lineHeight: "2.125rem" }],
        "ms-34": ["2.125rem", { lineHeight: "2.5rem" }],
        "ms-45": ["2.8125rem", { lineHeight: "3.25rem" }],
        "ms-57": ["3.5625rem", { lineHeight: "1.05" }],
        "ms-72": ["4.5rem", { lineHeight: "1.02" }],
      },
      maxWidth: {
        prose: "44rem",
      },
      boxShadow: {
        soft: "0 1px 2px hsl(20 14% 10% / 0.04), 0 8px 24px hsl(20 14% 10% / 0.06)",
        lift: "0 2px 4px hsl(20 14% 10% / 0.05), 0 18px 48px hsl(20 14% 10% / 0.10)",
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
        "fade-up": {
          from: { opacity: "0", transform: "translateY(12px)" },
          to: { opacity: "1", transform: "translateY(0)" },
        },
        "wave-rise": {
          "0%, 100%": { transform: "scaleY(0.35)" },
          "50%": { transform: "scaleY(1)" },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
        "pulse-record": "pulse-record 1.4s ease-in-out infinite",
        "fade-up": "fade-up 0.5s cubic-bezier(0.32, 0.72, 0, 1) both",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
};

export default config;
