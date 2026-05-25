import * as React from "react";

/**
 * Persistent reading-appearance preferences. The settings live in
 * localStorage so the application of the font-family / size / letter-
 * spacing happens entirely in the renderer — no Rust IPC churn for a
 * pure UI concern, the same pattern `use-theme` follows.
 *
 * Three knobs, all bundled locally so the app works offline:
 *  - `font`: System (SF Pro / Inter fallbacks), Fraunces, OpenDyslexic,
 *    Atkinson Hyperlegible.
 *  - `size`: S / M / L / XL, applied as a CSS variable that scales the
 *    base font-size from 14px → 18px.
 *  - `spacing`: Tight / Normal / Wide / Wider, applied as a CSS
 *    variable used as letter-spacing on the root.
 *
 * State is applied to `<html>` via three data attributes; the CSS in
 * globals.css selects on them and applies the actual font-family /
 * font-size / letter-spacing rules. Centralising the application path
 * keeps the runtime cost to a single React effect per setting change.
 *
 * v2 roadmap finding 100 (GET-113).
 */

export const READING_FONTS = [
  "system",
  "fraunces",
  "atkinson-hyperlegible",
  "opendyslexic",
] as const;
export type ReadingFont = (typeof READING_FONTS)[number];

export const READING_SIZES = ["s", "m", "l", "xl"] as const;
export type ReadingSize = (typeof READING_SIZES)[number];

export const READING_SPACINGS = ["tight", "normal", "wide", "wider"] as const;
export type ReadingSpacing = (typeof READING_SPACINGS)[number];

const STORAGE_FONT = "attune.reading.font";
const STORAGE_SIZE = "attune.reading.size";
const STORAGE_SPACING = "attune.reading.spacing";

const DEFAULTS: { font: ReadingFont; size: ReadingSize; spacing: ReadingSpacing } = {
  font: "system",
  size: "m",
  spacing: "normal",
};

function readStored<T extends string>(
  key: string,
  allowed: readonly T[],
  fallback: T
): T {
  if (typeof window === "undefined") return fallback;
  const raw = window.localStorage.getItem(key);
  return (allowed as readonly string[]).includes(raw ?? "") ? (raw as T) : fallback;
}

function applyToRoot(font: ReadingFont, size: ReadingSize, spacing: ReadingSpacing) {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.dataset.readingFont = font;
  root.dataset.readingSize = size;
  root.dataset.readingSpacing = spacing;
}

export function useReadingControls() {
  const [font, setFontState] = React.useState<ReadingFont>(() =>
    readStored(STORAGE_FONT, READING_FONTS, DEFAULTS.font)
  );
  const [size, setSizeState] = React.useState<ReadingSize>(() =>
    readStored(STORAGE_SIZE, READING_SIZES, DEFAULTS.size)
  );
  const [spacing, setSpacingState] = React.useState<ReadingSpacing>(() =>
    readStored(STORAGE_SPACING, READING_SPACINGS, DEFAULTS.spacing)
  );

  React.useEffect(() => {
    applyToRoot(font, size, spacing);
    window.localStorage.setItem(STORAGE_FONT, font);
    window.localStorage.setItem(STORAGE_SIZE, size);
    window.localStorage.setItem(STORAGE_SPACING, spacing);
  }, [font, size, spacing]);

  return {
    font,
    size,
    spacing,
    setFont: setFontState,
    setSize: setSizeState,
    setSpacing: setSpacingState,
  };
}

/**
 * Apply the saved reading preferences to `<html>` on first paint,
 * before React mounts. Matches the pattern `applyInitialTheme` uses
 * so the app doesn't flash a wrong font / size on cold start.
 */
export function applyInitialReadingControls() {
  applyToRoot(
    readStored(STORAGE_FONT, READING_FONTS, DEFAULTS.font),
    readStored(STORAGE_SIZE, READING_SIZES, DEFAULTS.size),
    readStored(STORAGE_SPACING, READING_SPACINGS, DEFAULTS.spacing)
  );
}
