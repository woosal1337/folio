import path from "node:path";
import { readFileSync } from "node:fs";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import svgr from "vite-plugin-svgr";

const pkg = JSON.parse(
  readFileSync(path.resolve(__dirname, "package.json"), "utf8")
) as {
  version: string;
};

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  plugins: [react(), svgr()],
  define: {
    __ATTUNE_VERSION__: JSON.stringify(pkg.version),
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari13",
    minify: (process.env.TAURI_ENV_DEBUG ? false : "esbuild") as "esbuild" | false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}));
