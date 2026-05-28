/**
 * GET-143 — Meeting-detection HUD.
 *
 * Renders inside the frameless, always-on-top `meeting-hud` window the
 * watcher opens when a conferencing app appears. Mirrors Granola's
 * detected-meeting popover: "Meeting detected — <App>" with a green
 * Take Notes button and a dropdown (Take Notes / Dismiss / Don't ask for
 * <App>). Auto-dismisses after a short delay and never steals focus —
 * the window owns the focus + always-on-top behaviour.
 */

import * as React from "react";
import { ChevronDown, Video, X } from "lucide-react";

import {
  dismissMeetingHud,
  getPendingMeeting,
  meetingTakeNotes,
  onMeetingDetected,
  suppressMeetingApp,
  type DetectedMeeting,
} from "@/shared/lib/ipc";

/** Auto-dismiss the HUD after this many ms of no interaction. */
const AUTO_DISMISS_MS = 12_000;

export default function MeetingHud() {
  const [meeting, setMeeting] = React.useState<DetectedMeeting | null>(null);
  const [menuOpen, setMenuOpen] = React.useState(false);

  // Initial read + live refresh while open.
  React.useEffect(() => {
    let unlisten: (() => void) | undefined;
    void getPendingMeeting()
      .then((m) => setMeeting(m))
      .catch(() => {});
    void onMeetingDetected((m) => setMeeting(m))
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {});
    return () => unlisten?.();
  }, []);

  // Auto-dismiss timer. Resets whenever a fresh detection arrives.
  React.useEffect(() => {
    if (!meeting) return;
    const id = window.setTimeout(() => {
      void dismissMeetingHud().catch(() => {});
    }, AUTO_DISMISS_MS);
    return () => window.clearTimeout(id);
  }, [meeting]);

  const onTakeNotes = React.useCallback(() => {
    void meetingTakeNotes().catch((e) => console.error("meeting_take_notes:", e));
  }, []);

  const onDismiss = React.useCallback(() => {
    void dismissMeetingHud().catch((e) => console.error("dismiss_meeting_hud:", e));
  }, []);

  const onSuppress = React.useCallback(() => {
    if (!meeting) return;
    void suppressMeetingApp(meeting.bundle_id).catch((e) =>
      console.error("suppress_meeting_app:", e)
    );
  }, [meeting]);

  const appName = meeting?.app_label ?? "a call";

  return (
    <div className="fixed inset-0 flex items-center gap-3 overflow-hidden rounded-xl border border-white/10 bg-neutral-900/95 px-3 py-2.5 text-white shadow-2xl backdrop-blur">
      <span className="relative flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-emerald-500/15 text-emerald-400">
        <Video className="h-4 w-4" />
        <span className="absolute -right-0.5 -top-0.5 h-2.5 w-2.5 animate-pulse rounded-full bg-emerald-400 ring-2 ring-neutral-900" />
      </span>

      <div className="min-w-0 flex-1 leading-tight">
        <p className="text-[10px] font-medium uppercase tracking-wider text-neutral-400">
          Meeting detected
        </p>
        <p className="truncate text-sm font-semibold">{appName}</p>
      </div>

      <div className="relative flex shrink-0 items-stretch">
        <button
          type="button"
          onClick={onTakeNotes}
          className="rounded-l-md bg-emerald-500 px-3 py-1.5 text-sm font-medium text-white transition-colors hover:bg-emerald-600"
        >
          Take Notes
        </button>
        <button
          type="button"
          aria-label="More options"
          aria-haspopup="menu"
          aria-expanded={menuOpen}
          onClick={() => setMenuOpen((v) => !v)}
          className="rounded-r-md border-l border-emerald-600/60 bg-emerald-500 px-1.5 text-white transition-colors hover:bg-emerald-600"
        >
          <ChevronDown className="h-3.5 w-3.5" />
        </button>

        {menuOpen ? (
          <div
            role="menu"
            className="absolute right-0 top-full z-10 mt-1 w-44 overflow-hidden rounded-md border border-white/10 bg-neutral-800 py-1 text-sm shadow-xl"
          >
            <MenuItem onClick={onTakeNotes}>Take Notes</MenuItem>
            <MenuItem onClick={onDismiss}>Dismiss</MenuItem>
            <MenuItem onClick={onSuppress}>{`Don't ask for ${appName}`}</MenuItem>
          </div>
        ) : null}
      </div>

      <button
        type="button"
        aria-label="Dismiss"
        onClick={onDismiss}
        className="shrink-0 rounded p-1 text-neutral-500 transition-colors hover:bg-white/10 hover:text-neutral-200"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

function MenuItem({
  onClick,
  children,
}: {
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onClick}
      className="block w-full px-3 py-1.5 text-left text-neutral-200 transition-colors hover:bg-white/10"
    >
      {children}
    </button>
  );
}
