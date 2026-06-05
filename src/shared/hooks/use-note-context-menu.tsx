import * as React from "react";
import { useNavigate } from "react-router-dom";
import {
  FileText,
  Folder,
  FolderInput,
  FolderOpen,
  FolderX,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";

import {
  clearRecordingArtifacts,
  deleteRecording,
  revealInFinder,
} from "@/shared/lib/ipc";
import { useFolders } from "@/shared/stores/folders-store";
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
  const folders = useFolders((s) => s.folders);
  const loadFolders = useFolders((s) => s.load);
  const assign = useFolders((s) => s.assign);
  const transcribe = useRecording((s) => s.transcribe);

  React.useEffect(() => void loadFolders(), [loadFolders]);

  return React.useCallback(
    (item: RecordingSummary, e: React.MouseEvent) => {
      e.preventDefault();
      const noteName =
        item.title?.trim() ||
        item.suggested_title?.trim() ||
        item.draft_name ||
        item.label;

      const move = async (folder: string | null) => {
        try {
          await assign(item.session_dir, folder);
          onChanged?.();
        } catch (err) {
          console.error("set_note_folder:", err);
          toast.error("Could not move note", { description: String(err) });
        }
      };

      const folderChildren: ContextMenuItem[] = [
        ...folders.map((f) => ({
          id: `mv:${f}`,
          label: f,
          icon: Folder,
          disabled: item.folder === f,
          onSelect: () => void move(f),
        })),
        ...(item.folder
          ? [
              {
                id: "mv:none",
                label: "Remove from folder",
                icon: FolderX,
                separatorBefore: folders.length > 0,
                onSelect: () => void move(null),
              },
            ]
          : []),
      ];

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
        {
          id: "move",
          label: "Move to folder",
          icon: FolderInput,
          disabled: folderChildren.length === 0,
          children: folderChildren.length ? folderChildren : undefined,
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
                      description: String(err),
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
              toast.error("Could not open Finder", { description: String(err) });
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
              toast.error("Could not delete note", { description: String(err) });
            }
          },
        },
      ];

      openMenu(e.clientX, e.clientY, items);
    },
    [navigate, openMenu, folders, assign, transcribe, onChanged]
  );
}
