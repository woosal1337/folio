import * as React from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";

import { createNote, renameNote } from "@/shared/lib/ipc";
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
  start: (sessionDir?: string) => Promise<void>,
  title?: string
) {
  void createNote()
    .then((note) => {
      // GET-161: pre-fill the title (e.g. the calendar meeting name) so
      // the note opens already named. Persist it and reflect it in the
      // router state the editor reads.
      // Guard against `onClick={takeNotes}` passing a MouseEvent as title.
      const named = typeof title === "string" ? title.trim() : undefined;
      const summary = named ? { ...note, title: named } : note;
      if (named) {
        void renameNote(note.session_dir, named).catch((e) =>
          console.error("rename_note:", e)
        );
      }
      navigate(`/editor/${encodeURIComponent(note.label)}`, {
        state: { recording: summary },
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

/** Create a note, open it, and start recording into it. An optional
 *  `title` pre-names the note (used by the Coming-up card, GET-161). */
export function useTakeNotes(): (title?: string) => void {
  const navigate = useNavigate();
  const start = useRecording((s) => s.start);
  return React.useCallback(
    (title?: string) => openNote(navigate, true, start, title),
    [navigate, start]
  );
}
