/**
 * GET-143 — Meeting-detection HUD.
 *
 * A pill that surfaces when a conferencing app is detected. GET-197 adds a
 * pre-meeting brief panel above the pill: 2-3 locally-generated bullets
 * that auto-recede after 8s so the user can read-and-forget.
 *
 * Window layout (196px tall, transparent):
 *   ┌──────────────────────────────┐ ← top (transparent when no brief)
 *   │  brief card (140px)          │
 *   │  • bullet 1                  │
 *   │  • bullet 2                  │
 *   │  • bullet 3                  │
 *   ├──────────────────────────────┤
 *   │  ● Meeting detected  [Take]  │ ← pill (56px)
 *   └──────────────────────────────┘
 */

import * as React from "react";
import { X } from "lucide-react";

import {
  dismissMeetingHud,
  getMeetingBrief,
  getPendingMeeting,
  meetingTakeNotes,
  nextCalendarEvent,
  onMeetingDetected,
  type BriefBullet,
  type DetectedMeeting,
} from "@/shared/lib/ipc";

/** Auto-dismiss the whole HUD after this many ms. */
const AUTO_DISMISS_MS = 14_000;
/** Brief panel fades out after this many ms (before the HUD auto-dismisses). */
const BRIEF_RECEDE_MS = 9_000;

export default function MeetingHud() {
  const [meeting, setMeeting] = React.useState<DetectedMeeting | null>(null);
  const [bullets, setBullets] = React.useState<BriefBullet[]>([]);
  const [briefVisible, setBriefVisible] = React.useState(true);
  const [sourcesCount, setSourcesCount] = React.useState(0);

  // Blank the window background so the transparent corners show through.
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

  // Fetch pre-meeting brief from the next calendar event (GET-197).
  React.useEffect(() => {
    if (!meeting) return;
    void (async () => {
      try {
        const event = await nextCalendarEvent();
        const attendees = event?.attendees ?? [];
        if (attendees.length === 0) return;
        const brief = await getMeetingBrief(attendees);
        if (brief && brief.bullets.length > 0) {
          setBullets(brief.bullets);
          setSourcesCount(brief.sources_count);
        }
      } catch {
        // Brief generation failed — HUD still works without it.
      }
    })();
  }, [meeting]);

  // Auto-dismiss timer — resets on new detection.
  React.useEffect(() => {
    if (!meeting) return;
    const id = window.setTimeout(() => {
      void dismissMeetingHud().catch(() => {});
    }, AUTO_DISMISS_MS);
    return () => window.clearTimeout(id);
  }, [meeting]);

  // Brief auto-recede — fades out after BRIEF_RECEDE_MS.
  React.useEffect(() => {
    if (bullets.length === 0) return;
    const id = window.setTimeout(() => setBriefVisible(false), BRIEF_RECEDE_MS);
    return () => window.clearTimeout(id);
  }, [bullets]);

  const onTakeNotes = React.useCallback(() => {
    void meetingTakeNotes().catch((e) => console.error("meeting_take_notes:", e));
  }, []);

  const onDismiss = React.useCallback(() => {
    void dismissMeetingHud().catch((e) => console.error("dismiss_meeting_hud:", e));
  }, []);

  const appName = meeting?.app_label ?? "a call";
  const showBrief = bullets.length > 0 && briefVisible;

  return (
    <div className="fixed inset-0 flex select-none flex-col justify-end overflow-hidden">
      {/* Brief card — slides in when bullets are ready (GET-197). */}
      <div
        aria-live="polite"
        style={{
          transition: "opacity 0.6s ease, transform 0.5s ease",
          opacity: showBrief ? 1 : 0,
          transform: showBrief ? "translateY(0)" : "translateY(-8px)",
          pointerEvents: showBrief ? "auto" : "none",
        }}
        className="mx-0.5 mb-1.5 overflow-hidden rounded-xl border border-white/10 bg-neutral-900/95 px-3.5 py-2.5 shadow-2xl backdrop-blur"
      >
        <p className="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-neutral-500">
          Before this meeting
        </p>
        <ul className="space-y-1.5">
          {bullets.map((b, i) => (
            <li
              key={i}
              className="flex items-start gap-2 text-[12px] leading-snug text-neutral-200"
            >
              <span
                className="mt-0.5 h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-500"
                aria-hidden
              />
              {b.text}
            </li>
          ))}
        </ul>
        {sourcesCount > 0 ? (
          <p className="mt-2 text-[10px] text-neutral-600">
            From {sourcesCount} local note{sourcesCount === 1 ? "" : "s"}
          </p>
        ) : null}
      </div>

      {/* Detection pill — unchanged from GET-143. */}
      <div
        className="flex items-center gap-2.5 overflow-hidden rounded-full border border-white/10 bg-neutral-900/95 px-3 text-white shadow-2xl backdrop-blur"
        style={{ height: 56 }}
      >
        <span
          className="relative flex h-7 w-7 shrink-0 items-center justify-center"
          aria-hidden="true"
        >
          <span className="absolute h-3.5 w-3.5 animate-ping rounded-full bg-emerald-500/40" />
          <span className="h-2 w-2 rounded-full bg-emerald-500" />
        </span>

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
    </div>
  );
}
