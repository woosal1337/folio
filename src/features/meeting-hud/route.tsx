import * as React from "react";
import { X } from "lucide-react";

import {
  dismissMeetingHud,
  getPendingMeeting,
  meetingTakeNotes,
  onMeetingDetected,
  type DetectedMeeting,
} from "@/shared/lib/ipc";

const AUTO_DISMISS_MS = 14_000;

export default function MeetingHud() {
  const [meeting, setMeeting] = React.useState<DetectedMeeting | null>(null);

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
    <div className="fixed inset-0 flex select-none flex-col justify-end overflow-hidden">
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
