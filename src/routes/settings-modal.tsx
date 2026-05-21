import * as React from "react";
import {
  Mic,
  Sparkles,
  Folder,
  Palette,
  KeyRound,
} from "lucide-react";

import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Separator } from "@/components/ui/separator";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useTheme } from "@/hooks/use-theme";
import { getSettings, listInputDevices, saveSettings } from "@/lib/api";
import type { DeviceInfo, Settings, Theme } from "@/lib/types";

type Section = "general" | "transcription" | "storage" | "appearance";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SettingsModal({ open, onOpenChange }: Props) {
  const [section, setSection] = React.useState<Section>("general");
  const [settings, setSettings] = React.useState<Settings | null>(null);
  const [devices, setDevices] = React.useState<DeviceInfo[]>([]);
  const [saving, setSaving] = React.useState(false);
  const { setTheme } = useTheme();

  // Load settings + devices once the modal opens.
  React.useEffect(() => {
    if (!open) return;
    let cancelled = false;
    (async () => {
      try {
        const [s, d] = await Promise.all([getSettings(), listInputDevices()]);
        if (cancelled) return;
        setSettings(s);
        setDevices(d);
      } catch (e) {
        console.error("settings load:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open]);

  const update = React.useCallback(
    <K extends keyof Settings>(key: K, value: Settings[K]) => {
      setSettings((s) => (s ? { ...s, [key]: value } : s));
    },
    []
  );

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await saveSettings(settings);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="grid h-[640px] max-h-[85vh] grid-cols-[200px_1fr] gap-0 overflow-hidden p-0">
        <DialogTitle className="sr-only">Settings</DialogTitle>
        <DialogDescription className="sr-only">
          Recording, transcription, storage, and appearance settings.
        </DialogDescription>

        {/* Left rail */}
        <div className="flex flex-col gap-1 border-r border-border bg-secondary p-3">
          <p className="px-3 pb-2 pt-1 text-2xs font-semibold uppercase tracking-wider text-muted-foreground">
            Settings
          </p>
          {(
            [
              { id: "general", label: "General", icon: Mic },
              { id: "transcription", label: "Transcription", icon: Sparkles },
              { id: "storage", label: "Storage", icon: Folder },
              { id: "appearance", label: "Appearance", icon: Palette },
            ] as { id: Section; label: string; icon: typeof Mic }[]
          ).map((item) => {
            const Icon = item.icon;
            const active = section === item.id;
            return (
              <button
                key={item.id}
                onClick={() => setSection(item.id)}
                className={cn(
                  "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium text-left transition-colors",
                  active
                    ? "bg-card text-foreground shadow-sm"
                    : "text-muted-foreground hover:bg-card/60 hover:text-foreground"
                )}
              >
                <Icon className="h-4 w-4 shrink-0" />
                {item.label}
              </button>
            );
          })}
        </div>

        {/* Right content */}
        <div className="flex h-full flex-col">
          <ScrollArea className="flex-1">
            <div className="px-8 py-7">
              {!settings ? (
                <p className="text-sm text-muted-foreground">Loading…</p>
              ) : section === "general" ? (
                <GeneralSection
                  settings={settings}
                  devices={devices}
                  onChange={update}
                />
              ) : section === "transcription" ? (
                <TranscriptionSection settings={settings} onChange={update} />
              ) : section === "storage" ? (
                <StorageSection settings={settings} />
              ) : (
                <AppearanceSection
                  theme={settings.theme}
                  onChange={(t) => {
                    update("theme", t);
                    setTheme(t);
                  }}
                />
              )}
            </div>
          </ScrollArea>

          <div className="flex items-center justify-end gap-2 border-t border-border bg-card px-6 py-3">
            <Button
              variant="ghost"
              onClick={() => onOpenChange(false)}
              disabled={saving}
            >
              Cancel
            </Button>
            <Button onClick={handleSave} disabled={saving || !settings}>
              {saving ? "Saving…" : "Save"}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

function GeneralSection({
  settings,
  devices,
  onChange,
}: {
  settings: Settings;
  devices: DeviceInfo[];
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}) {
  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">General</h2>

      <section className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Microphone
        </Label>
        <select
          value={settings.mic_device ?? ""}
          onChange={(e) =>
            onChange(
              "mic_device",
              e.target.value === "" ? null : e.target.value
            )
          }
          className="h-9 w-full rounded-md border border-input bg-card px-3 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <option value="">(system default)</option>
          {devices.map((d) => (
            <option key={d.name} value={d.name}>
              {d.name}
              {d.is_default ? " · default" : ""}
            </option>
          ))}
        </select>
        <p className="text-xs text-muted-foreground">
          {devices.length} input device{devices.length === 1 ? "" : "s"} visible
          to the system.
        </p>
      </section>

      <Separator />

      <section className="flex items-start justify-between gap-6">
        <div>
          <h3 className="font-medium">Capture system audio</h3>
          <p className="mt-1 max-w-md text-xs text-muted-foreground">
            Records what comes out of your speakers via ScreenCaptureKit. macOS
            prompts for Screen Recording permission the first time. Audio only —
            no video.
          </p>
        </div>
        <Switch
          checked={settings.system_audio_enabled}
          onCheckedChange={(v) => onChange("system_audio_enabled", v)}
        />
      </section>
    </div>
  );
}

function TranscriptionSection({
  settings,
  onChange,
}: {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}) {
  const providers: { id: Settings["transcriber"]; label: string; desc: string }[] = [
    {
      id: "openai",
      label: "OpenAI Whisper API",
      desc: "Uploaded to OpenAI · ~$0.006/min · multilingual",
    },
    {
      id: "local_whisper",
      label: "Local Whisper",
      desc: "Runs on this Mac · lands in a future session",
    },
  ];

  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Transcription</h2>

      <section className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Provider
        </Label>
        <div className="grid gap-2">
          {providers.map((p) => {
            const selected = settings.transcriber === p.id;
            return (
              <button
                key={p.id}
                onClick={() => onChange("transcriber", p.id)}
                className={cn(
                  "flex flex-col items-start gap-1 rounded-lg border p-4 text-left transition-colors",
                  selected
                    ? "border-primary bg-accent"
                    : "border-border bg-card hover:bg-secondary"
                )}
              >
                <div className="flex w-full items-center justify-between">
                  <span className="font-medium">{p.label}</span>
                  {selected && (
                    <Badge variant="accent" className="text-2xs">
                      selected
                    </Badge>
                  )}
                </div>
                <span className="text-xs text-muted-foreground">{p.desc}</span>
              </button>
            );
          })}
        </div>
      </section>

      {settings.transcriber === "openai" && (
        <section className="space-y-3">
          <Label
            htmlFor="openai-key"
            className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground"
          >
            <KeyRound className="h-3.5 w-3.5" />
            OpenAI API key
          </Label>
          <Input
            id="openai-key"
            type="password"
            placeholder="sk-..."
            value={settings.openai_api_key}
            onChange={(e) => onChange("openai_api_key", e.target.value)}
            className="font-mono"
          />
          <p className="text-xs text-muted-foreground">
            Stored locally. Sent only to api.openai.com.
          </p>
        </section>
      )}

      <section className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Language
        </Label>
        <select
          value={settings.transcription_language}
          onChange={(e) => onChange("transcription_language", e.target.value)}
          className="h-9 w-full rounded-md border border-input bg-card px-3 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <option value="auto">Auto-detect</option>
          <option value="en">English</option>
          <option value="tr">Turkish</option>
          <option value="az">Azerbaijani</option>
          <option value="ru">Russian</option>
          <option value="de">German</option>
          <option value="es">Spanish</option>
          <option value="fr">French</option>
          <option value="it">Italian</option>
          <option value="pt">Portuguese</option>
          <option value="ar">Arabic</option>
          <option value="ja">Japanese</option>
          <option value="zh">Chinese</option>
        </select>
        <p className="text-xs text-muted-foreground">
          Set a language if you record predominantly in one. Auto detects per
          segment.
        </p>
      </section>
    </div>
  );
}

function StorageSection({ settings }: { settings: Settings }) {
  const rows = [
    { label: "Recordings", value: settings.output_dir },
    { label: "Notes", value: settings.notes_dir },
    { label: "Transcripts", value: settings.transcripts_dir },
    { label: "Tasks", value: settings.tasks_path },
  ];

  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Storage</h2>
      <p className="text-sm text-muted-foreground">
        All paths are local. Folder pickers land in the next iteration.
      </p>
      <div className="grid gap-3">
        {rows.map((r) => (
          <div
            key={r.label}
            className="flex items-center justify-between gap-4 rounded-lg border border-border bg-card p-4"
          >
            <div>
              <p className="text-sm font-medium">{r.label}</p>
              <p className="mt-0.5 break-all font-mono text-xs text-muted-foreground">
                {r.value}
              </p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function AppearanceSection({
  theme,
  onChange,
}: {
  theme: Theme;
  onChange: (t: Theme) => void;
}) {
  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Appearance</h2>
      <p className="text-sm text-muted-foreground">
        Light is the default. Dark is available for late-night work.
      </p>
      <div className="grid grid-cols-2 gap-3">
        {(["light", "dark"] as Theme[]).map((t) => {
          const selected = theme === t;
          return (
            <button
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
    </div>
  );
}
