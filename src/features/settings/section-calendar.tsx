/**
 * GET-134 — Settings → Calendar.
 *
 * Display + per-calendar visibility. Mirrors Granola's layout with two
 * Attune deviations:
 *   1. The "Show events with no participants" toggle is hidden behind
 *      an Advanced disclosure (Tony — 95% of users don't need it).
 *   2. Per-calendar visibility uses smart defaults — when the backend
 *      lands, calendars matching the user's email domain are auto-
 *      enabled, personal/family/holiday calendars auto-disabled.
 *
 * The per-calendar list is a stub until the backend's calendar OAuth
 * + EventKit access endpoints ship (GET-122 epic Sprint 3).
 */

import * as React from "react";
import { Calendar, ChevronDown, ChevronRight, Eye, ExternalLink } from "lucide-react";

import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import type { Settings } from "@/shared/types/Settings";

interface SectionCalendarProps {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

export function SectionCalendar({ settings, onChange }: SectionCalendarProps) {
  const [showAdvanced, setShowAdvanced] = React.useState(false);

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Calendar</h2>
        <p className="text-sm text-muted-foreground">
          What Attune surfaces from your calendar and where.
        </p>
      </header>

      <DisplayGroup>
        <ToggleRow
          icon={Calendar}
          title="Show upcoming meetings in menu bar"
          description="Your next meeting and how long until it starts appear in the macOS menu bar."
          checked={settings.show_upcoming_meetings_in_menubar}
          onChange={(v) => onChange("show_upcoming_meetings_in_menubar", v)}
        />
        <button
          type="button"
          onClick={() => setShowAdvanced((s) => !s)}
          aria-expanded={showAdvanced}
          className="flex w-full items-center gap-1.5 rounded-md px-3 py-2 text-2xs uppercase tracking-wider text-muted-foreground hover:text-foreground"
        >
          {showAdvanced ? (
            <ChevronDown className="h-3.5 w-3.5" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5" />
          )}
          Advanced
        </button>
        {showAdvanced ? (
          <ToggleRow
            icon={Eye}
            title="Show events with no participants"
            description="Include focus blocks and solo events in the 'Coming up' menu-bar preview."
            checked={settings.show_events_without_participants}
            onChange={(v) => onChange("show_events_without_participants", v)}
          />
        ) : null}
      </DisplayGroup>

      <CalendarVisibilityStub />
    </section>
  );
}

function DisplayGroup({ children }: { children: React.ReactNode }) {
  return (
    <div className="space-y-3">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        Display
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

function CalendarVisibilityStub() {
  return (
    <div className="space-y-3">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        Visible calendars
      </Label>
      <div className="rounded-lg border border-dashed border-border bg-card p-5">
        <div className="flex items-start gap-3">
          <Calendar className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
          <div className="flex-1 space-y-1.5">
            <p className="text-sm font-medium">Connect a calendar to see it here</p>
            <p className="max-w-prose text-xs text-muted-foreground">
              Once you connect Google Calendar, Microsoft Outlook, or grant
              EventKit access in macOS, the calendars you choose to surface to
              Attune will list here. Smart defaults: any calendar whose recent
              events have attendees in your workspace domain is enabled
              automatically; personal / family / holiday calendars stay off.
            </p>
            <a
              href="https://attune.app/help/connect-calendar"
              className="inline-flex items-center gap-1 text-xs text-primary hover:underline"
            >
              See how to connect a calendar
              <ExternalLink className="h-3 w-3" />
            </a>
          </div>
        </div>
      </div>
    </div>
  );
}
