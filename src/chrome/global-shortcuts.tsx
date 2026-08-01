import * as React from "react";
import { useNavigate } from "react-router-dom";

import {
  SHORTCUTS,
  dispatch,
  focusInTextInput,
  matchesChord,
} from "@/shared/lib/shortcuts";
import { useRecording } from "@/shared/stores/recording-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";
import { useTakeNotes } from "@/shared/hooks/use-take-notes";

interface Props {
  onOpenCheatsheet: () => void;
  onOpenPalette: () => void;
}

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
          openPreferences,
          openCheatsheet: onOpenCheatsheet,
          openAsk: onOpenPalette,
          toggleRecording: () => {
            if (recording) void stop();
            else takeNotes();
          },
          segmentPrev: () => {
            document.dispatchEvent(new CustomEvent("folio:transcript-prev"));
          },
          segmentNext: () => {
            document.dispatchEvent(new CustomEvent("folio:transcript-next"));
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
