/**
 * Global Cmd-K command palette tokens. v2 finding 007 / GET-26.
 *
 * Spotlight-grade fuzzy palette across recordings, tasks, memories,
 * agent runs, and verbs. Doubles as entry to cross-library Ask.
 *
 * This module owns the source-agnostic data model + fuzzy scorer.
 * The palette component (Cmd-K overlay) consumes a `CommandSource`
 * per data type — those live in the route files that own the data.
 */

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
}

/**
 * Token-set fuzzy match score. Returns 0 when the query has tokens
 * not present in the haystack, otherwise a positive score that
 * rewards prefix matches and contiguous runs.
 *
 * Algorithm (Spotlight-flavoured, deliberately small):
 *   - lowercase everything, split query on whitespace.
 *   - every query token must appear in the haystack as a substring.
 *   - score += 100 per token that prefix-matches a haystack word.
 *   - score += 30 per token that matches anywhere.
 *   - score += 10 bonus when consecutive query tokens land in the
 *     same haystack word.
 */
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

/**
 * Rank a candidate set against `query`. Returns items sorted by
 * descending score with zero-score entries dropped. Stable: equal
 * scores keep their input order so the upstream "default" ordering
 * carries through.
 */
export function rank(items: CommandItem[], query: string): CommandItem[] {
  if (query.trim().length === 0) return items;
  const scored = items.map((item, idx) => {
    const hay = [item.title, item.subtitle ?? "", ...(item.keywords ?? [])].join(" ");
    return { item, score: scoreFuzzy(query, hay), idx };
  });
  return scored
    .filter((s) => s.score > 0)
    .sort((a, b) => (b.score - a.score) || (a.idx - b.idx))
    .map((s) => s.item);
}

/**
 * The verbs that always appear in the palette regardless of indexed
 * data: Start recording, Open Inbox, etc. Mirrors the keyboard
 * shortcut catalogue from #008 / GET-32 so a user who memorises
 * either surface stays consistent.
 */
export function verbSource(actions: {
  startRecording: () => void;
  openInbox: () => void;
  openLibrary: () => void;
  openMemory: () => void;
  openTasks: () => void;
  openPreferences: () => void;
  openCheatsheet: () => void;
}): CommandSource {
  return {
    kind: "verb",
    load: async () => [
      { id: "verb:record", kind: "verb", title: "Start recording", keywords: ["record", "capture", "meeting"], shortcut: "⌘R", action: actions.startRecording },
      { id: "verb:inbox", kind: "verb", title: "Open Inbox", keywords: ["today", "queue"], shortcut: "⌘2", action: actions.openInbox },
      { id: "verb:library", kind: "verb", title: "Open Library", keywords: ["recordings", "list"], shortcut: "⌘3", action: actions.openLibrary },
      { id: "verb:tasks", kind: "verb", title: "Open Tasks", keywords: ["kanban", "to-do"], shortcut: "⌘4", action: actions.openTasks },
      { id: "verb:memory", kind: "verb", title: "Open Memory", keywords: ["facts", "claims"], shortcut: "⌘5", action: actions.openMemory },
      { id: "verb:settings", kind: "verb", title: "Open Preferences", keywords: ["settings", "config"], shortcut: "⌘,", action: actions.openPreferences },
      { id: "verb:cheatsheet", kind: "verb", title: "Keyboard cheat sheet", keywords: ["shortcuts", "help"], shortcut: "⌘⇧/", action: actions.openCheatsheet },
    ],
  };
}
