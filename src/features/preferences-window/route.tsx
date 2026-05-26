import * as React from "react";
import { Bot, Crown, Folder, Lock, Mic, Palette, Plug, Sparkles, Waves } from "lucide-react";

import { ScrollArea } from "@/shared/ui/scroll-area";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { useTheme } from "@/shared/hooks/use-theme";
import { listInputDevices } from "@/shared/lib/ipc";
import type { DeviceInfo } from "@/shared/types/DeviceInfo";
import type { Settings } from "@/shared/types/Settings";

import { SectionAi } from "@/features/settings/section-ai";
import { SectionAppearance } from "@/features/settings/section-appearance";
import { SectionAudio } from "@/features/settings/section-audio";
import { SectionGeneral } from "@/features/settings/section-general";
import { SectionPrivacy } from "@/features/settings/section-privacy";
import { SectionPro } from "@/features/settings/section-pro";
import { SectionStorage } from "@/features/settings/section-storage";
import { SectionTranscription } from "@/features/settings/section-transcription";
import { SectionUsage } from "@/features/settings/section-usage";
import { SectionWebhooks } from "@/features/settings/section-webhooks";
import { SectionPermissions } from "@/features/settings/section-permissions";

type Section =
  | "general"
  | "appearance"
  | "audio"
  | "transcription"
  | "ai"
  | "privacy"
  | "permissions"
  | "storage"
  | "webhooks"
  | "pro"
  | "usage";

interface TabSpec {
  id: Section;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
}

const TABS: TabSpec[] = [
  { id: "general", label: "General", icon: Sparkles },
  { id: "appearance", label: "Appearance", icon: Palette },
  { id: "audio", label: "Audio", icon: Mic },
  { id: "transcription", label: "Transcription", icon: Waves },
  { id: "ai", label: "AI", icon: Bot },
  { id: "permissions", label: "Permissions", icon: Lock },
  { id: "privacy", label: "Privacy", icon: Lock },
  { id: "storage", label: "Storage", icon: Folder },
  { id: "webhooks", label: "Webhooks", icon: Plug },
  { id: "pro", label: "Pro", icon: Crown },
];

/**
 * Top-level route for the dedicated Preferences NSWindow. v2 finding
 * 020 / GET-86. Mounted at `/preferences-window`; the Rust side opens
 * a 640×520 native window pointed here, so macOS handles the chrome
 * (close / minimize / zoom) and the user gets a System-Settings-style
 * surface instead of an in-app modal.
 *
 * Re-uses the existing section components from the legacy
 * SettingsModal so the actual content stays in one place. The
 * sidebar + content-pane shell is duplicated here only to drop the
 * Dialog wrapper.
 */
export default function PreferencesWindow() {
  const settings = useSettingsStore((s) => s.settings);
  const load = useSettingsStore((s) => s.load);
  const save = useSettingsStore((s) => s.save);
  const { theme, setTheme } = useTheme();
  const [devices, setDevices] = React.useState<DeviceInfo[]>([]);
  const [section, setSection] = React.useState<Section>("general");

  React.useEffect(() => {
    if (!settings) load();
  }, [settings, load]);

  React.useEffect(() => {
    listInputDevices()
      .then(setDevices)
      .catch((e) => console.error("listInputDevices:", e));
  }, []);

  const onChange = React.useCallback(
    async <K extends keyof Settings>(key: K, value: Settings[K]) => {
      if (!settings) return;
      try {
        await save({ ...settings, [key]: value });
      } catch (e) {
        console.error("save settings:", e);
      }
    },
    [settings, save]
  );

  if (!settings) {
    return (
      <div className="flex h-screen items-center justify-center text-sm text-muted-foreground">
        Loading…
      </div>
    );
  }

  return (
    <div className="flex h-screen w-screen bg-background text-foreground">
      <aside className="flex w-44 shrink-0 flex-col gap-1 border-r border-border bg-sidebar p-3">
        {TABS.map((tab) => {
          const Icon = tab.icon;
          return (
            <Button
              key={tab.id}
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setSection(tab.id)}
              className={cn(
                "justify-start gap-2 text-sm",
                section === tab.id && "bg-accent text-accent-foreground"
              )}
            >
              <Icon className="h-3.5 w-3.5" />
              {tab.label}
            </Button>
          );
        })}
      </aside>
      <main className="flex-1 overflow-hidden">
        <ScrollArea className="h-full">
          <div className="px-8 py-8">
            {section === "general" ? (
              <SectionGeneral settings={settings} onChange={onChange} devices={devices} />
            ) : null}
            {section === "appearance" ? (
              <SectionAppearance theme={theme} onChange={setTheme} />
            ) : null}
            {section === "audio" ? <SectionAudio settings={settings} onChange={onChange} /> : null}
            {section === "transcription" ? (
              <SectionTranscription settings={settings} onChange={onChange} />
            ) : null}
            {section === "ai" ? <SectionAi settings={settings} onChange={onChange} /> : null}
            {section === "permissions" ? <SectionPermissions /> : null}
            {section === "privacy" ? (
              <SectionPrivacy settings={settings} onChange={onChange} />
            ) : null}
            {section === "storage" ? <SectionStorage settings={settings} onChange={onChange} /> : null}
            {section === "webhooks" ? <SectionWebhooks /> : null}
            {section === "pro" ? <SectionPro settings={settings} onChange={onChange} /> : null}
            {section === "usage" ? <SectionUsage /> : null}
          </div>
        </ScrollArea>
      </main>
    </div>
  );
}
