import * as React from "react";
import { Bell, Eye, Palette, Type } from "lucide-react";

import { Label } from "@/shared/ui/label";
import { useTheme, type Theme } from "@/shared/hooks/use-theme";
import {
  READING_FONTS,
  READING_SIZES,
  useReadingControls,
  type ReadingFont,
  type ReadingSize,
} from "@/shared/hooks/use-reading-controls";
import type { Settings } from "@/shared/types/Settings";

interface SectionPreferencesProps {
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

const FONT_LABELS: Record<ReadingFont, string> = {
  system: "System",
  fraunces: "Fraunces",
  "atkinson-hyperlegible": "Atkinson Hyperlegible",
  opendyslexic: "OpenDyslexic",
};

const SIZE_LABELS: Record<ReadingSize, string> = {
  s: "Small",
  m: "Medium",
  l: "Large",
  xl: "Extra Large",
};

export function SectionPreferences({ onChange }: SectionPreferencesProps) {
  const { theme, setTheme } = useTheme();
  const { font, size, setFont, setSize } = useReadingControls();

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Preferences</h2>
        <p className="text-sm text-muted-foreground">App-wide behaviour.</p>
      </header>

      <PreferencesGroup title="Appearance">
        <SelectRow
          icon={Palette}
          title="Theme"
          description="Choose how Folio looks. Matches your OS by default."
          value={theme}
          onChange={(v) => {
            onChange("theme", v);
            setTheme(v as Theme);
          }}
          options={[
            { value: "light", label: "Light" },
            { value: "dark", label: "Dark" },
          ]}
        />
        <SelectRow
          icon={Type}
          title="Reading font"
          description="Body font used for transcripts, notes, and summaries. Local fallbacks only — no network fetch."
          value={font}
          onChange={(v) => setFont(v as ReadingFont)}
          options={READING_FONTS.map((f) => ({ value: f, label: FONT_LABELS[f] }))}
        />
        <SelectRow
          icon={Eye}
          title="Reading size"
          description="Base type size for transcripts and notes."
          value={size}
          onChange={(v) => setSize(v as ReadingSize)}
          options={READING_SIZES.map((s) => ({ value: s, label: SIZE_LABELS[s] }))}
        />
      </PreferencesGroup>

      <PrivacyRedLineNotice />
    </section>
  );
}

function PreferencesGroup({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-3">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {title}
      </Label>
      <div className="space-y-2 rounded-lg border border-border bg-card p-2">
        {children}
      </div>
    </div>
  );
}

interface SelectRowProps {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}

function SelectRow({
  icon: Icon,
  title,
  description,
  value,
  onChange,
  options,
}: SelectRowProps) {
  const id = React.useId();
  return (
    <div className="flex items-start gap-4 rounded-md p-3 hover:bg-muted/30">
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1 space-y-0.5">
        <Label htmlFor={id} className="text-sm font-medium">
          {title}
        </Label>
        <p className="max-w-prose text-xs text-muted-foreground">{description}</p>
      </div>
      <select
        id={id}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="mt-1 h-8 shrink-0 rounded-md border border-input bg-card px-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </div>
  );
}

function PrivacyRedLineNotice() {
  return (
    <div
      className="mx-3 my-1 rounded-md border border-emerald-500/30 bg-emerald-500/5 px-4 py-3 text-2xs text-emerald-900 dark:text-emerald-200"
      role="note"
      aria-label="Folio privacy stance"
    >
      <p className="flex items-center gap-1.5 font-medium">
        <Bell className="h-3.5 w-3.5" />
        <span>What you won&apos;t see here</span>
      </p>
      <p className="mt-1.5 leading-relaxed">
        Folio does not collect transcripts to train models — there is no opt-out toggle
        because there is no collection.{" "}
        <span className="italic">
          Your meetings stay on your Mac unless you explicitly share them.
        </span>
      </p>
    </div>
  );
}
