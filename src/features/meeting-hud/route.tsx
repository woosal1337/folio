/**
 * GET-143 — Meeting-detection HUD.
 *
 * Renders inside the frameless, transparent, always-on-top `meeting-hud`
 * window the watcher opens when a user actually opens a mic stream
 * (joined a Discord channel, started a Zoom call, etc.). A one-row
 * capsule that matches the floating recording bar: same materials
 * (`bg-neutral-900/95`, `border-white/10`, `shadow-2xl backdrop-blur`)
 * and `rounded-full` so the "would you like to record" → "now recording"
 * transition reads as one widget morphing rather than two separate
 * notifications. Auto-dismisses after a short delay and never steals
 * focus — the window owns the focus + always-on-top behaviour.
 *
 * No dropdown menu in the pill (the recording bar has none either, and a
 * popover menu would either clip out of the 56-tall window or force a
 * tall transparent click-blocking gutter below the pill). Per-app
 * muting lives in the Notifications settings page.
 */

import * as React from "react";
import { X } from "lucide-react";

import {
  dismissMeetingHud,
  getPendingMeeting,
  meetingTakeNotes,
  onMeetingDetected,
  type DetectedMeeting,
} from "@/shared/lib/ipc";

/** Auto-dismiss the HUD after this many ms of no interaction. */
const AUTO_DISMISS_MS = 12_000;

export default function MeetingHud() {
  const [meeting, setMeeting] = React.useState<DetectedMeeting | null>(null);

  // The HUD window is transparent (see show_meeting_hud). The app's
  // <body> ships an opaque `bg-background` that would fill the window's
  // square corners and defeat the rounded pill — blank out the page
  // chrome in this dedicated window so only the capsule paints. Mirrors
  // the recording bar's body-blanking effect.
  React.useEffect(() => {
    const els = [document.documentElement, document.body];
    const prev = els.map((el) => el.style.background);
    els.forEach((el) => {
      el.style.background = "transparent";
    });
    return () => {
      els.forEach((el, i) => {
        el.style.background = prev[i] ?? "";
      });
    };
  }, []);

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

  const appName = meeting?.app_label ?? "a call";

  return (
    <div className="fixed inset-0 flex select-none items-center gap-2.5 overflow-hidden rounded-full border border-white/10 bg-neutral-900/95 px-3 text-white shadow-2xl backdrop-blur">
      {/* Pulse indicator — same affordance as the recording bar's red
          dot, in emerald to read as "ready / detected" rather than
          "live". The ping ring sells the "this is happening right now"
          read without animating any text. */}
      <span
        className="relative flex h-7 w-7 shrink-0 items-center justify-center"
        aria-hidden="true"
      >
        <span className="absolute h-3.5 w-3.5 animate-ping rounded-full bg-emerald-500/40" />
        <span className="h-2 w-2 rounded-full bg-emerald-500" />
      </span>

      {/* Single-line label keeps the pill silhouette clean. The label
          half stays neutral so the app name reads as the load-bearing
          content. */}
      <p className="min-w-0 flex-1 truncate text-[13px] leading-none">
        <span className="text-neutral-400">Meeting detected · </span>
        <span className="font-semibold text-white">{appName}</span>
      </p>

      <button
        type="button"
        onClick={onTakeNotes}
        className="shrink-0 rounded-full bg-emerald-500 px-3.5 py-1.5 text-xs font-semibold text-white transition-colors hover:bg-emerald-600"
      >
        Take Notes
      </button>

      <button
        type="button"
        aria-label="Dismiss"
        onClick={onDismiss}
        className="inline-flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-neutral-500 transition-colors hover:bg-white/10 hover:text-neutral-200"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
