import * as React from "react";
import { useEditor, EditorContent } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import Placeholder from "@tiptap/extension-placeholder";
import { Markdown } from "tiptap-markdown";

import { loadLiveNotes, saveLiveNotes } from "@/shared/lib/ipc";
import type { RawNoteLine } from "@/shared/types/RawNoteLine";

const AUTOSAVE_MS = 800;

interface Props {
  sessionDir: string | null;

  elapsedSeconds: number;

  disabled?: boolean;
}

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

export function MarkdownNotesEditor({
  sessionDir,
  elapsedSeconds,
  disabled = false,
}: Props) {
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
    editable: !disabled,
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

  React.useEffect(
    () => () => {
      if (saveTimer.current) window.clearTimeout(saveTimer.current);
      flush();
    },
    [flush]
  );

  React.useEffect(() => {
    if (!editor) return;
    if (disabled && saveTimer.current) {
      window.clearTimeout(saveTimer.current);
      flush();
    }
    editor.setEditable(!disabled);
  }, [editor, disabled, flush]);

  return (
    <EditorContent
      editor={editor}
      aria-disabled={disabled}
      className={
        "md-notes-editor transition-opacity" +
        (disabled ? " pointer-events-none select-none opacity-60" : "")
      }
    />
  );
}
