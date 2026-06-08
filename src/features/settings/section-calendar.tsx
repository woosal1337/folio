import * as React from "react";
import {
  Calendar,
  ChevronDown,
  ChevronRight,
  Eye,
  ExternalLink,
  Users,
  Video,
} from "lucide-react";

import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import {
  calendarAuthorizationStatus,
  listCalendarEvents,
  requestCalendarAccess,
} from "@/shared/lib/ipc";
import { humanizeError } from "@/shared/lib/errors";
import type { CalendarEvent } from "@/shared/types/CalendarEvent";
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
          What Folio surfaces from your calendar and where.
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

      <CalendarEventsPanel />
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

function CalendarEventsPanel() {
  const [status, setStatus] = React.useState<string | null>(null);
  const [events, setEvents] = React.useState<CalendarEvent[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<string | null>(null);

  const refresh = React.useCallback(async () => {
    try {
      const st = await calendarAuthorizationStatus();
      setStatus(st);
      setEvents(st === "authorized" ? await listCalendarEvents(14) : []);
      setError(null);
    } catch (e) {
      setError(humanizeError(e));
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    void refresh();
    const onFocus = () => void refresh();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refresh]);

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Upcoming events
        </Label>
        {status === "authorized" ? <ConnectedBadge /> : null}
      </div>
      {loading ? (
        <div className="rounded-lg border border-border bg-card px-4 py-5 text-xs text-muted-foreground">
          Loading your calendar…
        </div>
      ) : status !== "authorized" ? (
        <CalendarGrantPrompt status={status} onRefresh={refresh} />
      ) : events.length === 0 ? (
        <div className="rounded-lg border border-border bg-card px-4 py-5 text-xs text-muted-foreground">
          Connected, but no events in the next 14 days. Folio reads your macOS Calendar
          locally; nothing leaves your Mac.
        </div>
      ) : (
        <ul className="divide-y divide-border overflow-hidden rounded-lg border border-border bg-card">
          {events.map((e) => (
            <EventRow key={e.id} event={e} />
          ))}
        </ul>
      )}
      {error ? <p className="text-2xs text-destructive">{error}</p> : null}
    </div>
  );
}

function ConnectedBadge() {
  return (
    <span className="inline-flex items-center gap-1.5 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-2xs font-medium text-emerald-600 dark:text-emerald-400">
      <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
      Connected
    </span>
  );
}

function EventRow({ event }: { event: CalendarEvent }) {
  const start = new Date(event.starts_at);
  const end = new Date(event.ends_at);
  const day = start.toLocaleDateString([], {
    weekday: "short",
    month: "short",
    day: "numeric",
  });
  const time = (d: Date) =>
    d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  return (
    <li className="flex items-start gap-3 px-3 py-2.5">
      <Calendar className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">
          {event.title || "Untitled event"}
        </p>
        <p className="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-2xs text-muted-foreground">
          <span className="font-mono">
            {day} · {time(start)}–{time(end)}
          </span>
          {(event.attendees?.length ?? 0) > 0 ? (
            <span className="inline-flex items-center gap-1">
              <Users className="h-3 w-3" />
              {event.attendees?.length ?? 0}
            </span>
          ) : null}
          {event.location ? <span className="truncate">{event.location}</span> : null}
        </p>
      </div>
      {event.conference_url ? (
        <a
          href={event.conference_url}
          target="_blank"
          rel="noopener noreferrer"
          className="inline-flex shrink-0 items-center gap-1 rounded-md border border-border px-2 py-1 text-2xs font-medium text-primary hover:bg-accent"
        >
          <Video className="h-3 w-3" />
          Join
        </a>
      ) : null}
    </li>
  );
}

function CalendarGrantPrompt({
  status,
  onRefresh,
}: {
  status: string | null;
  onRefresh: () => Promise<void>;
}) {
  const [requesting, setRequesting] = React.useState(false);
  const denied = status === "denied" || status === "restricted";

  const grant = React.useCallback(async () => {
    setRequesting(true);
    try {
      await requestCalendarAccess();
      if (!denied) {
        for (let i = 0; i < 20; i += 1) {
          await new Promise((resolve) => setTimeout(resolve, 800));
          const next = await calendarAuthorizationStatus();
          if (next !== "not_determined") break;
        }
      }
      await onRefresh();
    } finally {
      setRequesting(false);
    }
  }, [denied, onRefresh]);

  return (
    <div className="rounded-lg border border-dashed border-border bg-card p-5">
      <div className="flex items-start gap-3">
        <Calendar className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
        <div className="flex-1 space-y-2.5">
          <p className="text-sm font-medium">
            {denied
              ? "Calendar access is turned off"
              : "Connect your calendar to see your events here"}
          </p>
          <p className="max-w-prose text-xs text-muted-foreground">
            Folio reads your macOS Calendar locally — whatever accounts you&apos;ve
            already added (iCloud, Google, Outlook, CalDAV) appear automatically. Folio
            does not connect to any cloud calendar itself; your calendar data stays on
            your Mac.
          </p>
          <div className="flex items-center gap-3 pt-0.5">
            <button
              type="button"
              onClick={() => void grant()}
              disabled={requesting}
              className="inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:cursor-not-allowed disabled:opacity-60"
            >
              {requesting
                ? "Waiting for macOS…"
                : denied
                  ? "Open System Settings"
                  : "Grant Calendar access"}
              {denied && !requesting ? <ExternalLink className="h-3 w-3" /> : null}
            </button>
            {denied ? null : (
              <a
                href="x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars"
                className="inline-flex items-center gap-1 text-2xs text-muted-foreground hover:text-foreground"
              >
                Open System Settings
                <ExternalLink className="h-3 w-3" />
              </a>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
