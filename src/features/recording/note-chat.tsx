/**
 * GET-150 — "Chat with this transcript" (per-note scoped chat).
 *
 * A conversation restricted to one meeting: its transcript, the live
 * notes, and any generated summary. Answers cite `[mm:ss]` timestamps
 * which render as clickable chips that seek the editor's audio player
 * (via the shared seek-audio event; a no-op where no player is mounted).
 *
 * Reused by the editor's "Chat with this transcript" affordance and the
 * post-recording bar's "Ask anything / What did I miss".
 */

import * as React from "react";
import { Loader2, MessageCircleQuestion, Send } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import {
  askNote,
  listChatThreads,
  saveChatThread,
  type ChatTurn,
} from "@/shared/lib/ipc";
import type { ChatThread } from "@/shared/types/ChatThread";
import { dispatchSeekAudio } from "@/features/editor/seek-audio";

interface Props {
  sessionDir: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Optional seed question (e.g. "What did I miss?"). */
  seed?: string;
}

/** Parse "1:02:05" / "12:34" into absolute seconds. */
function parseTimestamp(ts: string): number {
  const parts = ts.split(":").map((n) => parseInt(n, 10));
  if (parts.some((n) => Number.isNaN(n))) return 0;
  return parts.reduce((acc, n) => acc * 60 + n, 0);
}

const TS_RE = /\[(\d{1,2}:\d{2}(?::\d{2})?)\]/g;

/** Render assistant text, turning [mm:ss] citations into seek chips. */
function renderAnswer(text: string): React.ReactNode[] {
  const out: React.ReactNode[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  TS_RE.lastIndex = 0;
  let key = 0;
  while ((m = TS_RE.exec(text)) !== null) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const label = m[1] ?? "";
    const seconds = parseTimestamp(label);
    out.push(
      <button
        key={`ts-${key++}`}
        type="button"
        onClick={() => dispatchSeekAudio({ channel: "mic", seconds })}
        className="mx-0.5 rounded bg-primary/10 px-1 font-mono text-xs text-primary transition-colors hover:bg-primary/20"
        title="Jump the transcript player here"
      >
        {label}
      </button>
    );
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

interface Msg {
  role: "user" | "assistant";
  content: string;
}

export function NoteChat({ sessionDir, open, onOpenChange, seed }: Props) {
  const [messages, setMessages] = React.useState<Msg[]>([]);
  const [input, setInput] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const scrollRef = React.useRef<HTMLDivElement>(null);

  // Persisted per-note thread (GET-167): stable id + creation time so each
  // turn upserts the same file.
  const threadIdRef = React.useRef<string | null>(null);
  const createdAtRef = React.useRef<string | null>(null);

  // On open, restore this note's most recent conversation (if any) so the
  // history is there when the user comes back; seed the input otherwise.
  React.useEffect(() => {
    if (!open) return;
    let cancelled = false;
    listChatThreads("note", sessionDir)
      .then((threads) => {
        if (cancelled) return;
        const latest = threads[0];
        if (latest) {
          threadIdRef.current = latest.id;
          createdAtRef.current = latest.created_at;
          setMessages(
            latest.messages.map((m) => ({
              role: m.role as Msg["role"],
              content: m.content,
            }))
          );
        } else if (seed) {
          setInput(seed);
        }
      })
      .catch((e) => console.error("list_chat_threads:", e));
    return () => {
      cancelled = true;
    };
    // Only on open / note change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, sessionDir]);

  const persist = React.useCallback(
    (msgs: Msg[]) => {
      if (msgs.length === 0) return;
      if (!threadIdRef.current) {
        threadIdRef.current = (globalThis.crypto?.randomUUID?.() ??
          `t-${Date.now()}`) as string;
        createdAtRef.current = new Date().toISOString();
      }
      const firstUser = msgs.find((m) => m.role === "user")?.content ?? "Conversation";
      const title = firstUser.length > 60 ? `${firstUser.slice(0, 57)}…` : firstUser;
      const now = new Date().toISOString();
      const thread: ChatThread = {
        id: threadIdRef.current,
        scope: "note",
        session_dir: sessionDir,
        title,
        created_at: createdAtRef.current ?? now,
        updated_at: now,
        messages: msgs.map((m) => ({ role: m.role, content: m.content })),
      };
      saveChatThread(thread).catch((e) => console.error("save_chat_thread:", e));
    },
    [sessionDir]
  );

  React.useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [messages, busy]);

  const send = React.useCallback(async () => {
    const question = input.trim();
    if (!question || busy) return;
    const history: ChatTurn[] = messages.map((m) => ({
      role: m.role,
      content: m.content,
    }));
    setMessages((prev) => [...prev, { role: "user", content: question }]);
    setInput("");
    setBusy(true);
    try {
      const { answer } = await askNote(sessionDir, question, history);
      setMessages((prev) => {
        const next: Msg[] = [...prev, { role: "assistant", content: answer }];
        persist(next);
        return next;
      });
    } catch (e) {
      console.error("ask_note:", e);
      toast.error("Couldn't answer that", { description: String(e) });
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: "Sorry — I couldn't answer that just now." },
      ]);
    } finally {
      setBusy(false);
    }
  }, [input, busy, messages, sessionDir, persist]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="flex max-h-[80vh] max-w-2xl flex-col">
        <DialogHeader>
          <DialogTitle>Chat with this transcript</DialogTitle>
          <DialogDescription>
            Answers come only from this meeting. Click a timestamp to jump the player.
          </DialogDescription>
        </DialogHeader>

        <div
          ref={scrollRef}
          className="min-h-[12rem] flex-1 space-y-3 overflow-y-auto rounded-lg border border-border bg-card p-3"
        >
          {messages.length === 0 && !busy ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 py-8 text-center text-sm text-muted-foreground">
              <MessageCircleQuestion className="h-6 w-6" />
              <p>{`Ask anything about this meeting — "what did I miss?", "what did we decide?"`}</p>
            </div>
          ) : null}
          {messages.map((m, i) => (
            <div
              key={i}
              className={
                m.role === "user"
                  ? "ml-auto max-w-[85%] rounded-lg bg-primary px-3 py-2 text-sm text-primary-foreground"
                  : "mr-auto max-w-[85%] rounded-lg bg-muted px-3 py-2 text-sm leading-relaxed"
              }
            >
              {m.role === "assistant" ? renderAnswer(m.content) : m.content}
            </div>
          ))}
          {busy ? (
            <div className="mr-auto flex items-center gap-2 rounded-lg bg-muted px-3 py-2 text-sm text-muted-foreground">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              Thinking…
            </div>
          ) : null}
        </div>

        <form
          className="flex gap-2"
          onSubmit={(e) => {
            e.preventDefault();
            void send();
          }}
        >
          <input
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Ask about this meeting…"
            aria-label="Ask about this meeting"
            className="h-10 flex-1 rounded-md border border-input bg-card px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
          <Button type="submit" className="gap-2" disabled={busy || !input.trim()}>
            <Send className="h-4 w-4" />
            Ask
          </Button>
        </form>
      </DialogContent>
    </Dialog>
  );
}
