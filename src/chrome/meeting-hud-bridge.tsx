import * as React from "react";
import { useNavigate } from "react-router-dom";

import { onMeetingTakeNotes } from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";

/**
 * GET-143 — main-window listener for the meeting HUD's Take Notes
 * action. The HUD (a separate window) cannot drive this window's
 * recording store directly, so it emits `meeting:take-notes`; here we
 * navigate to the Record route and start capture through the store so
 * the ticker, tray title, and auto-transcribe-on-stop chain all engage.
 *
 * Headless — renders nothing. Mounted once inside the signed-in chrome,
 * alongside GlobalShortcuts, so `navigate` is available under the router.
 */
export function MeetingHudBridge() {
  const navigate = useNavigate();
  const start = useRecording((s) => s.start);

  React.useEffect(() => {
    let unlisten: (() => void) | undefined;
    void onMeetingTakeNotes(() => {
      navigate("/record");
      if (!useRecording.getState().recording) void start();
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((e) => console.error("onMeetingTakeNotes:", e));
    return () => unlisten?.();
  }, [navigate, start]);

  return null;
}
