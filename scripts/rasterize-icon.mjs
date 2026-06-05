import sharp from "sharp";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..");
const svgPath = resolve(root, "src-tauri/icons/icon-source.svg");
const outPath = resolve(root, "src-tauri/icons/logo-source.png");

const svg = readFileSync(svgPath);
await sharp(svg, { density: 384 })
  .resize(1024, 1024)
  .png({ compressionLevel: 9 })
  .toFile(outPath);

console.log(`rasterized ${svgPath} → ${outPath}`);
