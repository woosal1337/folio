import * as React from "react";
import { useNavigate } from "react-router-dom";
import { FileText, FolderOpen, RefreshCw, Trash2 } from "lucide-react";
import { toast } from "sonner";

import { humanizeError } from "@/shared/lib/errors";
import {
  clearRecordingArtifacts,
  deleteRecording,
  revealInFinder,
} from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";
import { confirmDelete } from "@/shared/stores/confirm-delete-store";
import {
  useContextMenu,
  type ContextMenuItem,
} from "@/shared/stores/context-menu-store";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

export function useNoteContextMenu(onChanged?: () => void) {
  const navigate = useNavigate();
  const openMenu = useContextMenu((s) => s.openMenu);
  const transcribe = useRecording((s) => s.transcribe);

  return React.useCallback(
    (item: RecordingSummary, e: React.MouseEvent) => {
      e.preventDefault();
      const noteName =
        item.title?.trim() ||
        item.suggested_title?.trim() ||
        item.draft_name ||
        item.label;

      const items: ContextMenuItem[] = [
        {
          id: "open",
          label: "Open",
          icon: FileText,
          onSelect: () =>
            navigate(`/editor/${encodeURIComponent(item.label)}`, {
              state: { recording: item },
            }),
        },
        ...(item.has_transcript
          ? [
              {
                id: "retr",
                label: "Re-transcribe",
                icon: RefreshCw,
                onSelect: async () => {
                  try {
                    await clearRecordingArtifacts(item.session_dir);
                    void transcribe(item.session_dir);
                    toast.success("Re-transcribing", { description: item.label });
                  } catch (err) {
                    console.error("re-transcribe:", err);
                    toast.error("Could not re-transcribe", {
                      description: humanizeError(err),
                    });
                  }
                },
              },
            ]
          : []),
        {
          id: "reveal",
          label: "Reveal in Finder",
          icon: FolderOpen,
          onSelect: () =>
            revealInFinder(item.session_dir).catch((err) => {
              console.error("reveal_in_finder:", err);
              toast.error("Could not open Finder", { description: humanizeError(err) });
            }),
        },
        {
          id: "del",
          label: "Delete note",
          icon: Trash2,
          destructive: true,
          separatorBefore: true,
          onSelect: async () => {
            const ok = await confirmDelete({
              title: "Delete this note?",
              description: `"${noteName}" — this removes the session folder and every file inside it (audio, transcript, notes). Cannot be undone.`,
              confirmLabel: "Delete note",
            });
            if (!ok) return;
            try {
              await deleteRecording(item.session_dir);
              onChanged?.();
              toast.success("Note deleted", { description: item.label });
            } catch (err) {
              console.error("delete_recording:", err);
              toast.error("Could not delete note", { description: humanizeError(err) });
            }
          },
        },
      ];

      openMenu(e.clientX, e.clientY, items);
    },
    [navigate, openMenu, transcribe, onChanged]
  );
}
