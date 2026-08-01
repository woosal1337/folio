import type { NavigateFunction } from "react-router-dom";

export type ShortcutAction =
  | "toggleRecording"
  | "openAsk"
  | "openCheatsheet"
  | "navHome"
  | "openPreferences"
  | "transcriptPrev"
  | "transcriptNext";

export interface Shortcut {
  action: ShortcutAction;
  label: string;
  group: "Recording" | "Navigation" | "Editing" | "Help";
  keys: KeyChord;
  enabledWhen?: "always" | "notInTextInput";
}

export interface KeyChord {
  key: string;
  cmd?: boolean;
  shift?: boolean;
  alt?: boolean;
  ctrl?: boolean;
}

export const SHORTCUTS: Shortcut[] = [
  {
    action: "toggleRecording",
    label: "Toggle recording",
    group: "Recording",
    keys: { key: "r", cmd: true },
  },
  {
    action: "openAsk",
    label: "Ask (chat)",
    group: "Recording",
    keys: { key: "k", cmd: true },
  },
  {
    action: "navHome",
    label: "Home",
    group: "Navigation",
    keys: { key: "1", cmd: true },
  },
  {
    action: "openPreferences",
    label: "Preferences",
    group: "Help",
    keys: { key: ",", cmd: true },
  },
  {
    action: "transcriptPrev",
    label: "Previous transcript segment",
    group: "Editing",
    keys: { key: "k" },
    enabledWhen: "notInTextInput",
  },
  {
    action: "transcriptNext",
    label: "Next transcript segment",
    group: "Editing",
    keys: { key: "j" },
    enabledWhen: "notInTextInput",
  },
  {
    action: "openCheatsheet",
    label: "Keyboard cheat sheet",
    group: "Help",
    keys: { key: "/", cmd: true, shift: true },
  },
];

export function formatChord(chord: KeyChord): string {
  const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
  const parts: string[] = [];
  if (chord.ctrl) parts.push(isMac ? "⌃" : "Ctrl");
  if (chord.alt) parts.push(isMac ? "⌥" : "Alt");
  if (chord.shift) parts.push(isMac ? "⇧" : "Shift");
  if (chord.cmd) parts.push(isMac ? "⌘" : "Ctrl");
  const key =
    chord.key === " "
      ? "Space"
      : chord.key.length === 1
        ? chord.key.toUpperCase()
        : chord.key;
  parts.push(key);
  return parts.join(isMac ? "" : "-");
}

export function matchesChord(event: KeyboardEvent, chord: KeyChord): boolean {
  const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
  const cmdKey = isMac ? event.metaKey : event.ctrlKey;
  if (!!chord.cmd !== cmdKey) return false;
  if (!!chord.shift !== event.shiftKey) return false;
  if (!!chord.alt !== event.altKey) return false;
  if (!!chord.ctrl !== (isMac ? event.ctrlKey : false)) return false;
  return event.key.toLowerCase() === chord.key.toLowerCase();
}

export function focusInTextInput(): boolean {
  const el = document.activeElement;
  if (!(el instanceof HTMLElement)) return false;
  if (el.isContentEditable) return true;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}

export function dispatch(
  action: ShortcutAction,
  ctx: {
    navigate: NavigateFunction;
    openPreferences: () => void;
    openCheatsheet: () => void;
    openAsk: () => void;
    toggleRecording: () => void;
    segmentPrev: () => void;
    segmentNext: () => void;
  }
): void {
  switch (action) {
    case "toggleRecording":
      ctx.toggleRecording();
      return;
    case "openAsk":
      ctx.openAsk();
      return;
    case "openCheatsheet":
      ctx.openCheatsheet();
      return;
    case "navHome":
      ctx.navigate("/");
      return;
    case "openPreferences":
      ctx.openPreferences();
      return;
    case "transcriptPrev":
      ctx.segmentPrev();
      return;
    case "transcriptNext":
      ctx.segmentNext();
      return;
  }
}
