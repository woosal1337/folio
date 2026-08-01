import * as React from "react";
import { useNavigate } from "react-router-dom";

import { onTrayEvent, onStitchingStarted, onStitchingDone } from "@/shared/lib/ipc";
import { registerNavigateFn } from "@/shared/lib/navigate-bridge";
import { useJobsStore } from "@/shared/stores/jobs-store";
import { useRecording } from "@/shared/stores/recording-store";
import { useTakeNotes } from "@/shared/hooks/use-take-notes";

const STITCHING_JOB_ID = "finalize:stitching";

export function EntryPointBridge() {
  const navigate = useNavigate();
  const takeNotes = useTakeNotes();
  const stop = useRecording((s) => s.stop);

  React.useEffect(() => {
    registerNavigateFn(navigate);
  }, [navigate]);

  React.useEffect(() => {
    const unlisteners: Array<() => void> = [];
    const track = (p: Promise<() => void>) =>
      void p
        .then((fn) => unlisteners.push(fn))
        .catch((e) => console.error("entry-point listener:", e));

    track(onTrayEvent("tray:start-recording", () => takeNotes()));
    track(
      onTrayEvent("tray:stop-recording", () => {
        if (useRecording.getState().recording) void stop();
      })
    );
    track(onTrayEvent("tray:open-library", () => navigate("/")));

    track(
      onStitchingStarted(() => {
        useJobsStore.getState().push({
          id: STITCHING_JOB_ID,
          kind: "finalize",
          label: "Stitching recording segments…",
        });
      })
    );
    track(
      onStitchingDone(() => {
        useJobsStore.getState().pop(STITCHING_JOB_ID);
      })
    );

    return () => unlisteners.forEach((fn) => fn());
  }, [navigate, takeNotes, stop]);

  return null;
}
