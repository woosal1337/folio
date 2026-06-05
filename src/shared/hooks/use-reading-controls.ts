import * as React from "react";

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

const STORAGE_FONT = "folio.reading.font";
const STORAGE_SIZE = "folio.reading.size";
const STORAGE_SPACING = "folio.reading.spacing";

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

export function applyInitialReadingControls() {
  applyToRoot(
    readStored(STORAGE_FONT, READING_FONTS, DEFAULTS.font),
    readStored(STORAGE_SIZE, READING_SIZES, DEFAULTS.size),
    readStored(STORAGE_SPACING, READING_SPACINGS, DEFAULTS.spacing)
  );
}
