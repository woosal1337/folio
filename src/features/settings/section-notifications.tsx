/**
 * GET-135 — Settings → Notifications.
 *
 * Three deviations from Granola's layout:
 *   1. Expanded default monitored-app list (Granola ships 6; we ship
 *      11 — Chrome, Safari, Firefox, Arc, Zoom, Teams, Meet, Webex,
 *      Slack huddles, Discord, FaceTime).
 *   2. Accessibility-API permission is optional, not required
 *      (Sasha). Fall back to NSRunningApplication bundle-identifier
 *      matching when the user denies the Accessibility prompt.
 *   3. Drop the marketing-emails section entirely — Attune does not
 *      send marketing email.
 *
 * Focus mode integration (Kenji) lives in the macOS layer via
 * UNUserNotificationCenter; nothing user-tweakable in this UI.
 */

import * as React from "react";
import { Bell, Inbox, Mic2, Video } from "lucide-react";
import {
  SiArc,
  SiDiscord,
  SiFirefoxbrowser,
  SiGooglechrome,
  SiGooglemeet,
  SiSafari,
  SiSlack,
  SiWebex,
  SiZoom,
} from "react-icons/si";
import { BsMicrosoftTeams } from "react-icons/bs";

import { Badge } from "@/shared/ui/badge";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import type { Settings } from "@/shared/types/Settings";

interface SectionNotificationsProps {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

type IconComponent = React.ComponentType<{ className?: string }>;

// Brand icons come from a maintained icon library (react-icons /
// simple-icons + bootstrap-icons for Teams). FaceTime has no brand glyph
// in the set (Apple), so it uses a neutral video icon.
const MONITORABLE_APPS: { bundleId: string; label: string; icon: IconComponent }[] = [
  { bundleId: "com.google.Chrome", label: "Chrome", icon: SiGooglechrome },
  { bundleId: "com.apple.Safari", label: "Safari", icon: SiSafari },
  { bundleId: "org.mozilla.firefox", label: "Firefox", icon: SiFirefoxbrowser },
  { bundleId: "company.thebrowser.Browser", label: "Arc", icon: SiArc },
  { bundleId: "us.zoom.xos", label: "Zoom", icon: SiZoom },
  {
    bundleId: "com.microsoft.teams2",
    label: "Microsoft Teams",
    icon: BsMicrosoftTeams,
  },
  { bundleId: "com.google.meetings", label: "Google Meet", icon: SiGooglemeet },
  { bundleId: "Cisco-Systems.Spark", label: "Webex", icon: SiWebex },
  { bundleId: "com.tinyspeck.slackmacgap", label: "Slack", icon: SiSlack },
  { bundleId: "com.hnc.Discord", label: "Discord", icon: SiDiscord },
  { bundleId: "com.apple.FaceTime", label: "FaceTime", icon: Video },
];

const NOTE_SHARED_OPTIONS: { value: string; label: string }[] = [
  { value: "activity_and_email", label: "Activity feed and email" },
  { value: "activity_only", label: "Activity feed only" },
  { value: "email_only", label: "Email only" },
  { value: "none", label: "Don't notify" },
];

export function SectionNotifications({
  settings,
  onChange,
}: SectionNotificationsProps) {
  const muted = React.useMemo(
    () => new Set(settings.notification_muted_apps),
    [settings.notification_muted_apps]
  );

  const toggleMuted = React.useCallback(
    (bundleId: string) => {
      const next = new Set(muted);
      if (next.has(bundleId)) next.delete(bundleId);
      else next.add(bundleId);
      onChange("notification_muted_apps", Array.from(next));
    },
    [muted, onChange]
  );

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Notifications</h2>
        <p className="text-sm text-muted-foreground">
          When Attune nudges you. Honours macOS Focus modes automatically.
        </p>
      </header>

      {/* Meeting notifications */}
      <Group title="Meeting notifications">
        <ToggleRow
          icon={Bell}
          title="Scheduled meetings"
          description="Notify 1 minute before meetings on your calendar start."
          checked={settings.notify_scheduled_meetings}
          onChange={(v) => onChange("notify_scheduled_meetings", v)}
        />
        <ToggleRow
          icon={Mic2}
          title="Auto-detected meetings"
          description="Notify when a call is detected in Zoom, Teams, Meet, or another conferencing app. Mute specific apps below."
          checked={settings.notify_auto_detected_meetings}
          onChange={(v) => onChange("notify_auto_detected_meetings", v)}
        />
        <div className="space-y-2 px-3 pb-3 pt-1">
          <Label className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
            {"Don't notify me in these apps"}
          </Label>
          <div className="flex flex-wrap gap-1.5">
            {MONITORABLE_APPS.map((app) => {
              const isMuted = muted.has(app.bundleId);
              const AppIcon = app.icon;
              return (
                <button
                  key={app.bundleId}
                  type="button"
                  onClick={() => toggleMuted(app.bundleId)}
                  aria-pressed={isMuted}
                  title={isMuted ? `${app.label} — muted` : app.label}
                  className={
                    "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs transition-colors " +
                    (isMuted
                      ? "border-amber-500/40 bg-amber-500/10 text-amber-800 dark:text-amber-200"
                      : "border-border bg-card text-muted-foreground hover:text-foreground")
                  }
                >
                  <AppIcon className="h-3.5 w-3.5 shrink-0" />
                  {app.label}
                </button>
              );
            })}
          </div>
          <p className="text-2xs text-muted-foreground">
            Detection uses NSRunningApplication bundle IDs. Granting macOS Accessibility
            access (System Settings → Privacy & Security → Accessibility) is optional
            and only sharpens detection — Attune works fine without it.
          </p>
        </div>
      </Group>

      {/* Shared notes */}
      <Group title="Shared notes">
        <SelectRow
          icon={Inbox}
          title="Someone shares a note with you"
          description="Where to surface 'a teammate shared a meeting note with you' events."
          value={settings.note_shared_notification}
          onChange={(v) => onChange("note_shared_notification", v)}
          options={NOTE_SHARED_OPTIONS}
        />
      </Group>

      {/* Marketing emails — explicit absence note. */}
      <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/5 px-4 py-3 text-2xs text-emerald-900 dark:text-emerald-200">
        <p className="flex items-center gap-1.5 font-medium">
          <Badge
            variant="outline"
            className="border-emerald-500/40 text-emerald-700 dark:text-emerald-300"
          >
            {"What's missing"}
          </Badge>
        </p>
        <p className="mt-1.5 leading-relaxed">
          There is no marketing-emails preference because Attune does not send marketing
          email. Account-critical mail (sign-in links, password resets, billing
          receipts) is the only thing you&apos;ll ever receive.
        </p>
      </div>
    </section>
  );
}

// ---------------------------------------------------------------
// Building blocks
// ---------------------------------------------------------------

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-3">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {title}
      </Label>
      <div className="space-y-1 rounded-lg border border-border bg-card p-2">
        {children}
      </div>
    </div>
  );
}

function ToggleRow({
  icon: Icon,
  title,
  description,
  checked,
  onChange,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
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

function SelectRow({
  icon: Icon,
  title,
  description,
  value,
  onChange,
  options,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
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
