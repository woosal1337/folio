#!/usr/bin/env node
/**
 * Motion lint (GET-200): ban layout/paint-thrashing CSS transitions.
 *
 * Only `transform` and `opacity` are compositor-only — animating
 * width/height/top/left/margin (or, in hot/looping UI, color/fill) forces
 * the browser to re-run layout or paint every frame. Granola measured a
 * single `height` transition at 60% CPU / 25% GPU on an M2; for an always-
 * recording, battery-sensitive app that's exactly the wrong default.
 *
 * Flags:
 *   - `transition-all` (Tailwind) — sweeps in layout/paint props.
 *   - `transition-[<layout-prop>]` (Tailwind arbitrary value).
 *   - CSS `transition: <layout-prop>` / `transition-property: <layout-prop>`.
 *
 * Escape hatch: put `motion-allow` in the offending line or the line above
 * it to justify a rare one-off (e.g. a sidebar collapse that genuinely
 * changes layout width and isn't hot/looping).
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { extname, join } from "node:path";

const ROOTS = ["src"];
const EXTS = new Set([".ts", ".tsx", ".css"]);
const LAYOUT_PAINT =
  "height|width|top|left|right|bottom|margin|padding|inset|fill";

const RULES = [
  {
    re: /\btransition-all\b/,
    why: "transition-all sweeps in layout/paint props — use transition-transform / transition-opacity / transition-colors",
  },
  {
    re: new RegExp(`\\btransition-\\[(?:${LAYOUT_PAINT})\\b`),
    why: "transitioning a layout property thrashes layout every frame — animate transform (translate/scale) instead",
  },
  {
    re: new RegExp(
      `\\btransition(?:-property)?\\s*:\\s*[^;{}]*\\b(?:${LAYOUT_PAINT})\\b`
    ),
    why: "CSS transition on a layout/paint property — animate transform/opacity instead",
  },
];

function walk(dir, out = []) {
  for (const name of readdirSync(dir)) {
    if (name === "node_modules" || name.startsWith(".")) continue;
    const p = join(dir, name);
    const s = statSync(p);
    if (s.isDirectory()) walk(p, out);
    else if (EXTS.has(extname(p))) out.push(p);
  }
  return out;
}

const offenders = [];
for (const root of ROOTS) {
  for (const file of walk(root)) {
    const lines = readFileSync(file, "utf8").split("\n");
    lines.forEach((line, i) => {
      const allowed =
        /motion-allow/.test(line) || (i > 0 && /motion-allow/.test(lines[i - 1]));
      if (allowed) return;
      for (const rule of RULES) {
        if (rule.re.test(line)) {
          offenders.push({ file, line: i + 1, text: line.trim().slice(0, 110), why: rule.why });
          break;
        }
      }
    });
  }
}

if (offenders.length > 0) {
  console.error(
    `\n✖ motion lint: ${offenders.length} layout/paint-thrashing animation(s) (GET-200)\n`
  );
  for (const o of offenders) {
    console.error(`  ${o.file}:${o.line}\n    ${o.text}\n    → ${o.why}\n`);
  }
  console.error(
    "Only animate transform + opacity in hot/looping UI. Justify a rare one-off with a `motion-allow` comment.\n"
  );
  process.exit(1);
}
console.log("✓ motion lint: no layout/paint-thrashing transitions");
