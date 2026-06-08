import * as React from "react";
import {
  BarChart3,
  Bell,
  Bot,
  Folder,
  Lock,
  Mic,
  Palette,
  Plug,
  Settings as SettingsIcon,
  Sparkles,
  Wallet,
  Waves,
  Workflow,
} from "lucide-react";
import { toast } from "sonner";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/shared/ui/dialog";
import { ScrollArea } from "@/shared/ui/scroll-area";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import { humanizeError } from "@/shared/lib/errors";
import { useTheme } from "@/shared/hooks/use-theme";
import { getSettings, listInputDevices } from "@/shared/lib/ipc";
import { useSettingsStore } from "@/shared/stores/settings-store";
import {
  useSettingsUiStore,
  type SettingsSection,
} from "@/shared/stores/settings-ui-store";
import type { DeviceInfo } from "@/shared/types/DeviceInfo";
import type { Settings } from "@/shared/types/Settings";

import { SectionAi } from "./section-ai";
import { SectionAppearance } from "./section-appearance";
import { SectionAudio } from "./section-audio";
import { SectionConnectors } from "./section-connectors";
import { SectionGeneral } from "./section-general";
import { SectionNotifications } from "./section-notifications";
import { SectionPreferences } from "./section-preferences";
import { SectionPrivacy } from "./section-privacy";
import { SectionStorage } from "./section-storage";
import { SectionTranscription } from "./section-transcription";
import { SectionUsage } from "./section-usage";
import { SectionWebhooks } from "./section-webhooks";
import { SectionAnalytics } from "./section-analytics";

type Section = SettingsSection;

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

interface NavItem {
  id: Section;
  label: string;
  icon: typeof Mic;
}

interface NavGroup {
  label: string;
  items: NavItem[];
}

const NAV_GROUPS: NavGroup[] = [
  {
    label: "Personal",
    items: [
      { id: "preferences", label: "Preferences", icon: SettingsIcon },
      { id: "analytics", label: "Analytics", icon: BarChart3 },
      { id: "notifications", label: "Notifications", icon: Bell },
    ],
  },
  {
    label: "Recording",
    items: [
      { id: "general", label: "General", icon: Mic },
      { id: "audio", label: "Audio", icon: Waves },
      { id: "transcription", label: "Transcription", icon: Sparkles },
      { id: "ai", label: "AI", icon: Bot },
      { id: "storage", label: "Storage", icon: Folder },
      { id: "privacy", label: "Privacy", icon: Lock },
      { id: "appearance", label: "Appearance", icon: Palette },
    ],
  },
  {
    label: "Integrations",
    items: [
      { id: "usage", label: "Usage", icon: Wallet },
      { id: "connectors", label: "Connectors", icon: Plug },
      { id: "webhooks", label: "Webhooks", icon: Workflow },
    ],
  },
];

export function SettingsModal({ open, onOpenChange }: Props) {
  const section = useSettingsUiStore((s) => s.section);
  const setSection = useSettingsUiStore((s) => s.setSection);
  const [settings, setSettings] = React.useState<Settings | null>(null);
  const [devices, setDevices] = React.useState<DeviceInfo[]>([]);
  const [saving, setSaving] = React.useState(false);
  const { theme: appTheme, setTheme } = useTheme();

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
        toast.error("Could not load settings", { description: humanizeError(e) });
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

  const saveToStore = useSettingsStore((s) => s.save);

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await saveToStore(settings);
      toast.success("Settings saved");
      onOpenChange(false);
    } catch (e) {
      console.error("settings save:", e);
      toast.error("Could not save settings", { description: humanizeError(e) });
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="grid h-[640px] max-h-[85vh] grid-cols-[200px_minmax(0,1fr)] gap-0 overflow-hidden p-0">
        <DialogTitle className="sr-only">Settings</DialogTitle>
        <DialogDescription className="sr-only">
          Recording, transcription, storage, and appearance settings.
        </DialogDescription>

        <nav
          aria-label="Settings sections"
          className="flex flex-col gap-3 overflow-y-auto border-r border-border bg-secondary p-3"
        >
          {NAV_GROUPS.map((group) => (
            <div key={group.label} className="flex flex-col gap-1">
              <p className="px-3 pb-1 pt-2 text-2xs font-semibold uppercase tracking-wider text-muted-foreground">
                {group.label}
              </p>
              {group.items.map((item) => {
                const Icon = item.icon;
                const active = section === item.id;
                return (
                  <button
                    type="button"
                    key={item.id}
                    onClick={() => setSection(item.id)}
                    aria-current={active ? "page" : undefined}
                    className={cn(
                      "flex items-center gap-3 rounded-md px-3 py-2 text-left text-sm font-medium transition-colors",
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
          ))}
        </nav>

        <div className="flex h-full min-h-0 min-w-0 flex-col overflow-hidden">
          <ScrollArea className="min-h-0 flex-1">
            <div className="px-8 py-7">
              {!settings ? (
                <p className="text-sm text-muted-foreground">Loading…</p>
              ) : section === "preferences" ? (
                <SectionPreferences settings={settings} onChange={update} />
              ) : section === "notifications" ? (
                <SectionNotifications settings={settings} onChange={update} />
              ) : section === "general" ? (
                <SectionGeneral
                  settings={settings}
                  devices={devices}
                  onChange={update}
                />
              ) : section === "audio" ? (
                <SectionAudio settings={settings} onChange={update} />
              ) : section === "transcription" ? (
                <SectionTranscription settings={settings} onChange={update} />
              ) : section === "ai" ? (
                <SectionAi settings={settings} onChange={update} />
              ) : section === "storage" ? (
                <SectionStorage settings={settings} onChange={update} />
              ) : section === "analytics" ? (
                <SectionAnalytics />
              ) : section === "connectors" ? (
                <SectionConnectors />
              ) : section === "webhooks" ? (
                <SectionWebhooks />
              ) : section === "usage" ? (
                <SectionUsage />
              ) : section === "privacy" ? (
                <SectionPrivacy settings={settings} onChange={update} />
              ) : (
                <SectionAppearance
                  theme={appTheme}
                  onChange={(t) => {
                    update("theme", t);
                    setTheme(t);
                  }}
                />
              )}
            </div>
          </ScrollArea>

          <div className="flex shrink-0 items-center justify-end gap-2 border-t border-border bg-card px-6 py-3">
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
