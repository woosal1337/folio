/// <reference types="vite/client" />
/// <reference types="vite-plugin-svgr/client" />

declare module "*.css";
declare module "*.png" {
  const src: string;
  export default src;
}
declare module "*.svg" {
  const src: string;
  export default src;
}
declare module "*.jpg" {
  const src: string;
  export default src;
}

// Build-time constants injected by vite.config.ts (`define`).
declare const __ATTUNE_VERSION__: string;
