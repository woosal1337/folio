export type CommandKind =
  | "recording"
  | "task"
  | "memory"
  | "agent-run"
  | "decision"
  | "verb";

export interface CommandItem {
  id: string;
  kind: CommandKind;
  title: string;
  subtitle?: string;
  keywords?: string[];
  shortcut?: string;
  action: () => void | Promise<void>;
}

export interface CommandSource {
  kind: CommandKind;
  load: () => Promise<CommandItem[]>;

  search?: (query: string) => Promise<CommandItem[]>;
}

export function scoreFuzzy(query: string, haystack: string): number {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return 1;
  const h = haystack.toLowerCase();
  const tokens = q.split(/\s+/).filter(Boolean);
  if (tokens.length === 0) return 1;
  const words = h.split(/[\s\-_/]+/).filter(Boolean);
  let score = 0;
  let lastWordIdx = -1;
  for (const token of tokens) {
    const prefixIdx = words.findIndex((w) => w.startsWith(token));
    if (prefixIdx >= 0) {
      score += 100;
      if (lastWordIdx >= 0 && prefixIdx === lastWordIdx + 1) score += 10;
      lastWordIdx = prefixIdx;
      continue;
    }
    if (h.includes(token)) {
      score += 30;
      lastWordIdx = -1;
      continue;
    }
    return 0;
  }
  return score;
}

export function rank(items: CommandItem[], query: string): CommandItem[] {
  if (query.trim().length === 0) return items;
  const scored = items.map((item, idx) => {
    const hay = [item.title, item.subtitle ?? "", ...(item.keywords ?? [])].join(" ");
    return { item, score: scoreFuzzy(query, hay), idx };
  });
  return scored
    .filter((s) => s.score > 0)
    .sort((a, b) => b.score - a.score || a.idx - b.idx)
    .map((s) => s.item);
}

export function verbSource(actions: {
  startRecording: () => void;
  openChat: () => void;
  openLibrary: () => void;
  openPreferences: () => void;
  openCheatsheet: () => void;
}): CommandSource {
  return {
    kind: "verb",
    load: async () => [
      {
        id: "verb:record",
        kind: "verb",
        title: "Start recording",
        keywords: ["record", "capture", "meeting"],
        shortcut: "⌘R",
        action: actions.startRecording,
      },
      {
        id: "verb:chat",
        kind: "verb",
        title: "Open Chat",
        keywords: ["ask", "chat", "todos", "recap", "coach"],
        action: actions.openChat,
      },
      {
        id: "verb:library",
        kind: "verb",
        title: "Open Home",
        keywords: ["home", "notes", "recordings", "list", "library"],
        shortcut: "⌘1",
        action: actions.openLibrary,
      },
      {
        id: "verb:settings",
        kind: "verb",
        title: "Open Preferences",
        keywords: ["settings", "config"],
        shortcut: "⌘,",
        action: actions.openPreferences,
      },
      {
        id: "verb:cheatsheet",
        kind: "verb",
        title: "Keyboard cheat sheet",
        keywords: ["shortcuts", "help"],
        shortcut: "⌘⇧/",
        action: actions.openCheatsheet,
      },
    ],
  };
}
