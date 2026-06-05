import * as React from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";

import { createNote, renameNote } from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";

function openNote(
  navigate: ReturnType<typeof useNavigate>,
  record: boolean,
  start: (sessionDir?: string) => Promise<void>,
  title?: string
) {
  void createNote()
    .then((note) => {
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

export function useQuickNote(): () => void {
  const navigate = useNavigate();
  const start = useRecording((s) => s.start);
  return React.useCallback(() => openNote(navigate, false, start), [navigate, start]);
}

export function useTakeNotes(): (title?: string) => void {
  const navigate = useNavigate();
  const start = useRecording((s) => s.start);
  return React.useCallback(
    (title?: string) => openNote(navigate, true, start, title),
    [navigate, start]
  );
}
