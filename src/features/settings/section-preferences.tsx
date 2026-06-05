import * as React from "react";
import {
  Bell,
  Eye,
  Languages,
  Link as LinkIcon,
  LogIn,
  Palette,
  ShieldCheck,
  Sparkles,
  Trash2,
  Type,
  Users,
} from "lucide-react";

import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
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
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

const LINK_SHARING_OPTIONS: { value: string; label: string; description: string }[] = [
  {
    value: "disabled",
    label: "Disabled",
    description: "Sharing links is turned off. Notes stay on your Mac.",
  },
  {
    value: "anyone_with_link",
    label: "Anyone with the link",
    description: "Anyone who has the URL can open it. Use with care.",
  },
];

const AUTO_DELETE_OPTIONS: {
  value: number | null;
  label: string;
  description?: string;
}[] = [
  { value: 7, label: "7 days" },
  { value: 30, label: "30 days" },
  {
    value: 90,
    label: "90 days",
    description: "Recommended — GDPR data-minimisation default.",
  },
  { value: 365, label: "1 year" },
  {
    value: null,
    label: "Off",
    description: "Keep transcripts indefinitely. Not recommended in the EU.",
  },
];

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

export function SectionPreferences({ settings, onChange }: SectionPreferencesProps) {
  const { theme, setTheme } = useTheme();
  const { font, size, setFont, setSize } = useReadingControls();

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Preferences</h2>
        <p className="text-sm text-muted-foreground">App-wide behaviour.</p>
      </header>

      <PreferencesGroup title="General">
        <ToggleRow
          icon={Sparkles}
          title="Live meeting indicator"
          description="A thin indicator on the right edge of the screen while Folio is transcribing."
          checked={settings.live_meeting_indicator}
          onChange={(v) => onChange("live_meeting_indicator", v)}
        />
        <ToggleRow
          icon={LogIn}
          title="Open Folio when you log in"
          description="Folio launches automatically when you sign into macOS."
          checked={settings.open_at_login}
          onChange={(v) => onChange("open_at_login", v)}
        />
        <ToggleRow
          icon={Users}
          title="Move Folio aside in meetings"
          description="When a meeting starts, Folio repositions so you can keep typing notes alongside your conferencing app."
          checked={settings.move_aside_in_meetings}
          onChange={(v) => onChange("move_aside_in_meetings", v)}
        />
      </PreferencesGroup>

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

      <PreferencesGroup title="Privacy">
        <SelectRow
          icon={LinkIcon}
          title="Default link sharing"
          description="Who can open a shared meeting link by default. You can override per share."
          value={settings.default_link_sharing}
          onChange={(v) => onChange("default_link_sharing", v)}
          options={LINK_SHARING_OPTIONS.map((o) => ({
            value: o.value,
            label: o.label,
          }))}
          longDescription={
            LINK_SHARING_OPTIONS.find((o) => o.value === settings.default_link_sharing)
              ?.description
          }
        />
        <ToggleRow
          icon={LinkIcon}
          title="Always open shared links in Folio"
          description="When you click a shared meeting link in your browser, jump into the Folio app instead of opening a web view."
          checked={settings.always_open_shared_links}
          onChange={(v) => onChange("always_open_shared_links", v)}
        />
        <ToggleRow
          icon={ShieldCheck}
          title="Privacy tier colour band"
          description="Coloured left border on every artefact (green = on-device, amber = encrypted cloud, red = third-party cloud)."
          checked={settings.privacy_tier_band_enabled}
          onChange={(v) => onChange("privacy_tier_band_enabled", v)}
        />
        <SelectRow
          icon={Trash2}
          title="Auto-delete transcripts"
          description="Older transcripts are removed automatically. GDPR Art. 5 data minimisation default."
          value={autoDeleteValue(settings.auto_delete_period_days)}
          onChange={(v) => onChange("auto_delete_period_days", autoDeletePersist(v))}
          options={AUTO_DELETE_OPTIONS.map((o) => ({
            value: autoDeleteValue(o.value),
            label: o.label,
          }))}
          longDescription={
            AUTO_DELETE_OPTIONS.find(
              (o) => o.value === settings.auto_delete_period_days
            )?.description
          }
        />
        <PrivacyRedLineNotice />
      </PreferencesGroup>
    </section>
  );
}

function autoDeleteValue(v: number | null | undefined): string {
  return v === null || v === undefined ? "off" : String(v);
}

function autoDeletePersist(v: string): number | null {
  return v === "off" ? null : Number.parseInt(v, 10);
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

interface ToggleRowProps {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}

function ToggleRow({
  icon: Icon,
  title,
  description,
  checked,
  onChange,
}: ToggleRowProps) {
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
      <Switch
        id={id}
        checked={checked}
        onCheckedChange={onChange}
        className="mt-1 shrink-0"
      />
    </div>
  );
}

interface SelectRowProps {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  longDescription?: string;
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}

function SelectRow({
  icon: Icon,
  title,
  description,
  longDescription,
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
        {longDescription ? (
          <p className="max-w-prose text-2xs italic text-muted-foreground">
            {longDescription}
          </p>
        ) : null}
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

void Languages;
