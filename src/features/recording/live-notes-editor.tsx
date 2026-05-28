/**
 * GET-145 — live note-taking editor.
 *
 * A free-form notes pane the user types in *while* mic + system capture
 * runs. Each line is anchored to the recording timestamp at which it was
 * created and can be a plain note or a `/action` `/decision` `/question`
 * `/highlight` command. The raw buffer autosaves into the session dir
 * (atomic) as the user types and repopulates on resume; the grouped
 * markdown render (Action items / Decisions / Open questions /
 * Highlights / Notes) is produced by the Rust `live_notes` module and
 * written next to it as the saved note.
 */

import * as React from "react";
import { ListChecks } from "lucide-react";

import { loadLiveNotes, saveLiveNotes } from "@/shared/lib/ipc";
import type { RawNoteLine } from "@/shared/types/RawNoteLine";

const AUTOSAVE_MS = 800;

const SLASH_COMMANDS: { token: string; label: string; hint: string }[] = [
  { token: "/action", label: "Action item", hint: "a task to follow up" },
  { token: "/decision", label: "Decision", hint: "something agreed" },
  { token: "/question", label: "Open question", hint: "to resolve later" },
  { token: "/highlight", label: "Highlight", hint: "a key moment" },
];

interface Props {
  /** Live session directory to persist into; null disables persistence. */
  sessionDir: string | null;
  /** Current recording-relative elapsed seconds, for anchoring new lines. */
  elapsedSeconds: number;
}

export function LiveNotesEditor({ sessionDir, elapsedSeconds }: Props) {
  const [lines, setLines] = React.useState<RawNoteLine[]>([]);
  const [menuQuery, setMenuQuery] = React.useState<string | null>(null);
  const textareaRef = React.useRef<HTMLTextAreaElement>(null);

  // Keep a live ref of the latest lines + dir so the debounced autosave
  // and the unmount flush always see the freshest values.
  const linesRef = React.useRef(lines);
  linesRef.current = lines;
  const dirRef = React.useRef(sessionDir);
  dirRef.current = sessionDir;
  const elapsedRef = React.useRef(elapsedSeconds);
  elapsedRef.current = elapsedSeconds;

  const value = React.useMemo(() => lines.map((l) => l.text).join("\n"), [lines]);

  // Load any existing notes when the session dir becomes known (resume
  // mid-recording, or re-open after a window reload).
  React.useEffect(() => {
    if (!sessionDir) return;
    let cancelled = false;
    void loadLiveNotes(sessionDir)
      .then((loaded) => {
        if (!cancelled && loaded.length > 0) setLines(loaded);
      })
      .catch((e) => console.error("load_live_notes:", e));
    return () => {
      cancelled = true;
    };
  }, [sessionDir]);

  const flush = React.useCallback(() => {
    const dir = dirRef.current;
    if (!dir) return;
    void saveLiveNotes(dir, linesRef.current).catch((e) =>
      console.error("save_live_notes:", e)
    );
  }, []);

  // Debounced autosave on every edit.
  React.useEffect(() => {
    if (!sessionDir) return;
    const id = window.setTimeout(flush, AUTOSAVE_MS);
    return () => window.clearTimeout(id);
  }, [lines, sessionDir, flush]);

  // Final flush on unmount (e.g. the user hits Stop) so the last few
  // keystrokes inside the debounce window are not lost.
  React.useEffect(() => () => flush(), [flush]);

  const reconcile = React.useCallback((next: string) => {
    const texts = next.split("\n");
    setLines((prev) =>
      texts.map((text, i) => {
        const old = prev[i];
        if (old)
          return old.text === text ? old : { text, anchor_seconds: old.anchor_seconds };
        return { text, anchor_seconds: elapsedRef.current };
      })
    );
  }, []);

  // Detect a slash-command being typed on the active line: the line
  // under the cursor is `/` followed by an optional partial word.
  const refreshMenu = React.useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    const upto = el.value.slice(0, el.selectionStart ?? 0);
    const activeLine = upto.slice(upto.lastIndexOf("\n") + 1);
    const m = /^\/(\w*)$/.exec(activeLine);
    setMenuQuery(m ? (m[1] ?? "") : null);
  }, []);

  const completeCommand = React.useCallback(
    (token: string) => {
      const el = textareaRef.current;
      if (!el) return;
      const all = el.value;
      const caret = el.selectionStart ?? all.length;
      const lineStart = all.lastIndexOf("\n", Math.max(0, caret - 1)) + 1;
      const next = `${all.slice(0, lineStart)}${token} ${all.slice(caret)}`;
      reconcile(next);
      setMenuQuery(null);
      // Restore focus + place the cursor after the inserted "token ".
      window.requestAnimationFrame(() => {
        el.focus();
        const pos = lineStart + token.length + 1;
        el.setSelectionRange(pos, pos);
      });
    },
    [reconcile]
  );

  const visibleCommands = React.useMemo(() => {
    if (menuQuery === null) return [];
    return SLASH_COMMANDS.filter((c) => c.token.slice(1).startsWith(menuQuery));
  }, [menuQuery]);

  return (
    <div className="flex w-full max-w-3xl flex-col gap-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm font-medium text-foreground">
          <ListChecks className="h-4 w-4 text-muted-foreground" />
          Notes
        </div>
        <div className="flex flex-wrap gap-1">
          {SLASH_COMMANDS.map((c) => (
            <button
              key={c.token}
              type="button"
              onClick={() => completeCommand(c.token)}
              title={c.hint}
              className="rounded-full border border-border bg-card px-2 py-0.5 text-2xs text-muted-foreground transition-colors hover:text-foreground"
            >
              {c.token}
            </button>
          ))}
        </div>
      </div>

      <div className="relative">
        <textarea
          ref={textareaRef}
          value={value}
          onChange={(e) => {
            reconcile(e.target.value);
            refreshMenu();
          }}
          onKeyUp={refreshMenu}
          onClick={refreshMenu}
          onBlur={() => {
            setMenuQuery(null);
            flush();
          }}
          placeholder="Type notes as the meeting runs. Start a line with / for action items, decisions, questions, highlights."
          aria-label="Live meeting notes"
          rows={8}
          className="w-full resize-y rounded-lg border border-border bg-card px-3 py-2.5 text-sm leading-relaxed shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />

        {visibleCommands.length > 0 ? (
          <div
            role="menu"
            className="absolute left-3 top-full z-10 mt-1 w-56 overflow-hidden rounded-md border border-border bg-popover py-1 text-sm shadow-lg"
          >
            {visibleCommands.map((c) => (
              <button
                key={c.token}
                type="button"
                role="menuitem"
                // onMouseDown (not onClick) so it fires before the
                // textarea's onBlur clears the menu.
                onMouseDown={(e) => {
                  e.preventDefault();
                  completeCommand(c.token);
                }}
                className="flex w-full items-baseline gap-2 px-3 py-1.5 text-left transition-colors hover:bg-accent hover:text-accent-foreground"
              >
                <span className="font-medium">{c.token}</span>
                <span className="text-2xs text-muted-foreground">{c.hint}</span>
              </button>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}
