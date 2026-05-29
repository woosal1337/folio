/**
 * Markdown notes editor — the user's personal notes pane.
 *
 * A live WYSIWYG markdown editor (TipTap): typing markdown syntax
 * (`# `, `**bold**`, `- `, `>`, `` `code` ``…) auto-converts and styles
 * inline as you write. The document is stored as markdown text, autosaved
 * into the session dir as `live_notes.json` (one line per `RawNoteLine`,
 * anchored to the recording position when capturing) so it stays the
 * input the on-stop summary folds in (GET-147). Replaces the older
 * slash-command textarea.
 */

import * as React from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import { Markdown } from "tiptap-markdown";

import { loadLiveNotes, saveLiveNotes } from "@/shared/lib/ipc";
import type { RawNoteLine } from "@/shared/types/RawNoteLine";

const AUTOSAVE_MS = 800;

interface Props {
  /** Live session directory to persist into; null disables persistence. */
  sessionDir: string | null;
  /** Current recording-relative elapsed seconds, for anchoring new lines. */
  elapsedSeconds: number;
}

/** Split a markdown string into `RawNoteLine[]`, preserving prior line
 *  anchors by index and stamping new lines with the current elapsed. */
function toLines(
  markdown: string,
  prev: RawNoteLine[],
  elapsed: number
): RawNoteLine[] {
  return markdown.split("\n").map((text, i) => {
    const old = prev[i];
    if (old)
      return old.text === text ? old : { text, anchor_seconds: old.anchor_seconds };
    return { text, anchor_seconds: elapsed };
  });
}

export function MarkdownNotesEditor({ sessionDir, elapsedSeconds }: Props) {
  const linesRef = React.useRef<RawNoteLine[]>([]);
  const dirRef = React.useRef(sessionDir);
  dirRef.current = sessionDir;
  const elapsedRef = React.useRef(elapsedSeconds);
  elapsedRef.current = elapsedSeconds;
  const saveTimer = React.useRef<number | null>(null);

  const flush = React.useCallback(() => {
    const dir = dirRef.current;
    if (!dir) return;
    void saveLiveNotes(dir, linesRef.current).catch((e) =>
      console.error("save_live_notes:", e)
    );
  }, []);

  const scheduleSave = React.useCallback(
    (markdown: string) => {
      linesRef.current = toLines(markdown, linesRef.current, elapsedRef.current);
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(flush, AUTOSAVE_MS);
    },
    [flush]
  );

  const editor = useEditor({
    immediatelyRender: false,
    extensions: [
      StarterKit,
      Placeholder.configure({
        placeholder:
          "Type your notes. Markdown works — # heading, **bold**, - list, > quote.",
      }),
      Markdown.configure({ html: false, linkify: true }),
    ],
    editorProps: {
      attributes: {
        "aria-label": "Meeting notes",
        class:
          "min-h-[8rem] rounded-lg border border-border bg-card px-3 py-2.5 text-sm leading-relaxed shadow-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-ring",
      },
    },
    onUpdate: ({ editor }) => {
      scheduleSave(editor.storage.markdown.getMarkdown());
    },
  });

  // Load existing notes once the session dir is known (re-open / resume).
  React.useEffect(() => {
    if (!sessionDir || !editor) return;
    let cancelled = false;
    void loadLiveNotes(sessionDir)
      .then((loaded) => {
        if (cancelled || loaded.length === 0) return;
        linesRef.current = loaded;
        editor.commands.setContent(loaded.map((l) => l.text).join("\n"));
      })
      .catch((e) => console.error("load_live_notes:", e));
    return () => {
      cancelled = true;
    };
  }, [sessionDir, editor]);

  // Flush the last edits on unmount (e.g. the user hits Stop).
  React.useEffect(
    () => () => {
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
      flush();
    },
    [flush]
  );

  return <EditorContent editor={editor} className="md-notes-editor" />;
}
