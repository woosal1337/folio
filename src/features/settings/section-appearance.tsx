import type * as React from "react";

import { cn } from "@/shared/lib/utils";
import type { Theme } from "@/shared/hooks/use-theme";
import {
  type ReadingFont,
  type ReadingSize,
  type ReadingSpacing,
  useReadingControls,
} from "@/shared/hooks/use-reading-controls";

interface Props {
  theme: Theme;
  onChange: (t: Theme) => void;
}

const FONT_OPTIONS: { id: ReadingFont; label: string; sample: string }[] = [
  { id: "system", label: "System", sample: "SF Pro / Inter" },
  { id: "fraunces", label: "Fraunces", sample: "Modern serif" },
  {
    id: "atkinson-hyperlegible",
    label: "Atkinson Hyperlegible",
    sample: "Low-vision friendly",
  },
  { id: "opendyslexic", label: "OpenDyslexic", sample: "Dyslexia friendly" },
];

const SIZE_OPTIONS: { id: ReadingSize; label: string; px: string }[] = [
  { id: "s", label: "S", px: "14px" },
  { id: "m", label: "M", px: "16px" },
  { id: "l", label: "L", px: "17px" },
  { id: "xl", label: "XL", px: "18px" },
];

const SPACING_OPTIONS: { id: ReadingSpacing; label: string }[] = [
  { id: "tight", label: "Tight" },
  { id: "normal", label: "Normal" },
  { id: "wide", label: "Wide" },
  { id: "wider", label: "Wider" },
];

export function SectionAppearance({ theme, onChange }: Props) {
  const reading = useReadingControls();
  return (
    <div className="flex flex-col gap-10">
      <section className="flex flex-col gap-5">
        <div>
          <h2 className="font-serif text-2xl font-medium">Appearance</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Light is the default. Dark is available for late-night work.
          </p>
        </div>
        <div className="grid grid-cols-2 gap-3">
          {(["light", "dark"] as Theme[]).map((t) => {
            const selected = theme === t;
            return (
              <button
                type="button"
                key={t}
                onClick={() => onChange(t)}
                className={cn(
                  "flex flex-col items-start gap-2 rounded-lg border p-4 text-left transition-colors",
                  selected
                    ? "border-primary bg-accent"
                    : "border-border bg-card hover:bg-secondary"
                )}
              >
                <div
                  className={cn(
                    "h-16 w-full rounded-md border",
                    t === "light"
                      ? "border-zinc-200 bg-[#F5F2EC]"
                      : "border-zinc-700 bg-[#0d0d10]"
                  )}
                />
                <span className="font-medium capitalize">{t}</span>
              </button>
            );
          })}
        </div>
      </section>

      <section className="flex flex-col gap-5">
        <div>
          <h2 className="font-serif text-2xl font-medium">Reading</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Tune the typography across the app for comfortable reading. All fonts are
            bundled locally — nothing is fetched from the network.
          </p>
        </div>

        <FieldGroup label="Font family">
          <div className="grid grid-cols-2 gap-2">
            {FONT_OPTIONS.map((opt) => {
              const selected = reading.font === opt.id;
              return (
                <button
                  type="button"
                  key={opt.id}
                  onClick={() => reading.setFont(opt.id)}
                  aria-pressed={selected}
                  data-font-preview={opt.id}
                  className={cn(
                    "flex flex-col items-start gap-1 rounded-lg border px-4 py-3 text-left transition-colors",
                    selected
                      ? "border-primary bg-accent"
                      : "border-border bg-card hover:bg-secondary"
                  )}
                >
                  <span className="text-sm font-medium">{opt.label}</span>
                  <span className="text-2xs text-muted-foreground">{opt.sample}</span>
                </button>
              );
            })}
          </div>
        </FieldGroup>

        <FieldGroup label="Text size">
          <div className="grid grid-cols-4 gap-2">
            {SIZE_OPTIONS.map((opt) => {
              const selected = reading.size === opt.id;
              return (
                <button
                  type="button"
                  key={opt.id}
                  onClick={() => reading.setSize(opt.id)}
                  aria-pressed={selected}
                  className={cn(
                    "flex flex-col items-center gap-0.5 rounded-lg border px-3 py-2.5 transition-colors",
                    selected
                      ? "border-primary bg-accent"
                      : "border-border bg-card hover:bg-secondary"
                  )}
                >
                  <span className="text-sm font-medium tabular-nums">{opt.label}</span>
                  <span className="text-2xs tabular-nums text-muted-foreground">
                    {opt.px}
                  </span>
                </button>
              );
            })}
          </div>
        </FieldGroup>

        <FieldGroup label="Letter spacing">
          <div className="grid grid-cols-4 gap-2">
            {SPACING_OPTIONS.map((opt) => {
              const selected = reading.spacing === opt.id;
              return (
                <button
                  type="button"
                  key={opt.id}
                  onClick={() => reading.setSpacing(opt.id)}
                  aria-pressed={selected}
                  className={cn(
                    "rounded-lg border px-3 py-2.5 text-sm transition-colors",
                    selected
                      ? "border-primary bg-accent"
                      : "border-border bg-card hover:bg-secondary"
                  )}
                >
                  {opt.label}
                </button>
              );
            })}
          </div>
        </FieldGroup>

        <p
          className="rounded-md border border-border bg-card px-4 py-3 text-sm text-muted-foreground"
          aria-live="polite"
        >
          The quick brown fox jumps over the lazy dog — 0123456789. Sample text reflects
          your current selection.
        </p>
      </section>
    </div>
  );
}

interface FieldGroupProps {
  label: string;
  children: React.ReactNode;
}

function FieldGroup({ label, children }: FieldGroupProps) {
  return (
    <div className="flex flex-col gap-2">
      <p className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        {label}
      </p>
      {children}
    </div>
  );
}
