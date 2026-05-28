import * as React from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";

import { createNote } from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";

/**
 * GET-155 — note-first flows.
 *
 * In the Granola-clean model there is no Record screen: every entry
 * point creates a note and opens it. Recording (when wanted) attaches to
 * that note's directory.
 *
 * - {@link useQuickNote}: create an empty note and open it (no capture).
 * - {@link useTakeNotes}: create a note, open it, and start capturing
 *   into it. The meeting HUD, the menu-bar tray, and Cmd-R converge here.
 *
 * Both must be called from inside the router so `navigate` is available.
 */

function openNote(
  navigate: ReturnType<typeof useNavigate>,
  record: boolean,
  start: (sessionDir?: string) => Promise<void>
) {
  void createNote()
    .then((note) => {
      navigate(`/editor/${encodeURIComponent(note.label)}`, {
        state: { recording: note },
      });
      if (record) {
        const { recording, busy } = useRecording.getState();
        if (!recording && !busy) void start(note.session_dir);
      }
    })
    .catch((e) => {
      console.error("create_note:", e);
      toast.error("Couldn't create a note", { description: String(e) });
    });
}

/** Create an empty note and open it — no recording. */
export function useQuickNote(): () => void {
  const navigate = useNavigate();
  const start = useRecording((s) => s.start);
  return React.useCallback(() => openNote(navigate, false, start), [navigate, start]);
}

/** Create a note, open it, and start recording into it. */
export function useTakeNotes(): () => void {
  const navigate = useNavigate();
  const start = useRecording((s) => s.start);
  return React.useCallback(() => openNote(navigate, true, start), [navigate, start]);
}
