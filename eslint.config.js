import js from "@eslint/js";
import reactPlugin from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
import jsxA11y from "eslint-plugin-jsx-a11y";
import tsParser from "@typescript-eslint/parser";
import tsPlugin from "@typescript-eslint/eslint-plugin";

export default [
  {
    ignores: [
      "dist/**",
      "target/**",
      "src-tauri/gen/**",
      "src-tauri/target/**",
      "node_modules/**",
      "scripts/**/*.mjs",
      "src/shared/types/**",
    ],
  },
  js.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 2022,
        sourceType: "module",
        ecmaFeatures: { jsx: true },
        project: "./tsconfig.json",
      },
      globals: {
        window: "readonly",
        document: "readonly",
        console: "readonly",
        setTimeout: "readonly",
        clearTimeout: "readonly",
        setInterval: "readonly",
        clearInterval: "readonly",
        HTMLElement: "readonly",
        HTMLInputElement: "readonly",
        HTMLButtonElement: "readonly",
        HTMLDivElement: "readonly",
        HTMLAudioElement: "readonly",
        HTMLImageElement: "readonly",
        Element: "readonly",
        Node: "readonly",
        Event: "readonly",
        MouseEvent: "readonly",
        KeyboardEvent: "readonly",
        AbortController: "readonly",
        AbortSignal: "readonly",
        fetch: "readonly",
        URL: "readonly",
        URLSearchParams: "readonly",
        crypto: "readonly",
        localStorage: "readonly",
        sessionStorage: "readonly",
        navigator: "readonly",
        location: "readonly",
        history: "readonly",
      },
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
      react: reactPlugin,
      "react-hooks": reactHooks,
      "jsx-a11y": jsxA11y,
    },
    settings: {
      react: { version: "detect" },
    },
    rules: {
      ...tsPlugin.configs.recommended.rules,
      ...reactPlugin.configs.recommended.rules,
      ...reactPlugin.configs["jsx-runtime"].rules,
      ...reactHooks.configs.recommended.rules,
      ...jsxA11y.configs.recommended.rules,

      "no-unused-vars": "off",
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/consistent-type-imports": [
        "error",
        { prefer: "type-imports", fixStyle: "inline-type-imports" },
      ],
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-non-null-assertion": "warn",

      "react/prop-types": "off",
      "react/react-in-jsx-scope": "off",
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",

      "jsx-a11y/no-autofocus": "warn",
      // The following three rules are temporarily relaxed. Phase 3 of the
      // architecture refactor (feature-based frontend + Zustand stores)
      // replaces window.confirm with accessible Dialog, adds keyboard
      // handlers to chrome regions, and tightens these back to "error".
      "jsx-a11y/click-events-have-key-events": "off",
      "jsx-a11y/no-static-element-interactions": "off",
      "jsx-a11y/media-has-caption": "off",

      "no-console": ["warn", { allow: ["warn", "error", "info"] }],
      eqeqeq: ["error", "always"],
      "prefer-const": "error",
      "no-var": "error",
      "object-shorthand": ["error", "always"],
    },
  },
];
