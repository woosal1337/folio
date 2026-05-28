import * as React from "react";
import { useNavigate } from "react-router-dom";

import {
  SHORTCUTS,
  dispatch,
  focusInTextInput,
  matchesChord,
} from "@/shared/lib/shortcuts";
import { openPreferencesWindow } from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";
import { useTakeNotes } from "@/shared/hooks/use-take-notes";

interface Props {
  onOpenCheatsheet: () => void;
  onOpenPalette: () => void;
}

/**
 * App-level keyboard listener. Reads the SHORTCUTS catalogue and
 * dispatches each chord through the same code path the cheatsheet
 * documents. Suppresses single-letter chords (J / K, no modifier)
 * while focus is inside a text input.
 *
 * Mounted once inside the router so dispatch can call navigate().
 * Ask / new-task / segment-prev-next currently route to no-op
 * placeholders; the dedicated panes wire their real handlers when
 * they ship.
 */
export function GlobalShortcuts({ onOpenCheatsheet, onOpenPalette }: Props) {
  const navigate = useNavigate();
  const openPreferences = useSettingsUiStore((s) => s.openAt);
  const recording = useRecording((s) => s.recording);
  const stop = useRecording((s) => s.stop);
  const takeNotes = useTakeNotes();

  React.useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      for (const shortcut of SHORTCUTS) {
        if (!matchesChord(event, shortcut.keys)) continue;
        if (shortcut.enabledWhen === "notInTextInput" && focusInTextInput()) continue;
        event.preventDefault();
        dispatch(shortcut.action, {
          navigate,
          openPreferences: () => {
            openPreferencesWindow().catch((e) => {
              console.error("open_preferences_window:", e);
              openPreferences();
            });
          },
          openCheatsheet: onOpenCheatsheet,
          openAsk: onOpenPalette,
          toggleRecording: () => {
            if (recording) void stop();
            else takeNotes();
          },
          newTask: () => navigate("/tasks"),
          segmentPrev: () => {
            document.dispatchEvent(new CustomEvent("attune:transcript-prev"));
          },
          segmentNext: () => {
            document.dispatchEvent(new CustomEvent("attune:transcript-next"));
          },
        });
        return;
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [
    navigate,
    openPreferences,
    onOpenCheatsheet,
    onOpenPalette,
    recording,
    takeNotes,
    stop,
  ]);

  return null;
}
