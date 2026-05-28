import * as React from "react";
import { useNavigate } from "react-router-dom";

import { onMeetingTakeNotes, onTrayEvent } from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";
import { useTakeNotes } from "@/shared/hooks/use-take-notes";

/**
 * GET-144 — converges every "start a meeting note" entry point onto the
 * single take-notes flow, and wires the rest of the menu-bar tray menu.
 *
 * Entry points handled:
 *   - meeting HUD Take Notes (`meeting:take-notes`, GET-143)
 *   - tray "Start Recording" (`tray:start-recording`)
 *   - tray "Stop Recording" / "Open Library" / "Open Inbox"
 *
 * Cmd-R converges via the same {@link useTakeNotes} hook inside
 * GlobalShortcuts. Headless — renders nothing. Mounted once inside the
 * signed-in chrome so `navigate` is available under the router.
 */
export function EntryPointBridge() {
  const navigate = useNavigate();
  const takeNotes = useTakeNotes();
  const stop = useRecording((s) => s.stop);

  React.useEffect(() => {
    const unlisteners: Array<() => void> = [];
    const track = (p: Promise<() => void>) =>
      void p
        .then((fn) => unlisteners.push(fn))
        .catch((e) => console.error("entry-point listener:", e));

    track(onMeetingTakeNotes(() => takeNotes()));
    track(onTrayEvent("tray:start-recording", () => takeNotes()));
    track(
      onTrayEvent("tray:stop-recording", () => {
        if (useRecording.getState().recording) void stop();
      })
    );
    track(onTrayEvent("tray:open-library", () => navigate("/library")));
    track(onTrayEvent("tray:open-inbox", () => navigate("/inbox")));

    return () => unlisteners.forEach((fn) => fn());
  }, [navigate, takeNotes, stop]);

  return null;
}
