import * as React from "react";
import { useLocation } from "react-router-dom";
import {
  CalendarRange,
  Compass,
  Eye,
  ListTodo,
  Loader2,
  Plus,
  Send,
  Sparkles,
  Trash2,
} from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { humanizeError } from "@/shared/lib/errors";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Markdown } from "@/shared/ui/markdown";
import {
  askLibrary,
  deleteChatThread,
  listChatThreads,
  listProviderModels,
  listRecipes,
  saveChatThread,
  type ChatTurn,
  type CoverageNote,
  type UserRecipe,
} from "@/shared/lib/ipc";
import { confirmDelete } from "@/shared/stores/confirm-delete-store";
import type { ChatThread } from "@/shared/types/ChatThread";
import type { ModelInfo } from "@/shared/types/ModelInfo";

interface Recipe {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  prompt: string;

  userDefined?: boolean;
}

function iconForName(
  name: string | null | undefined
): React.ComponentType<{ className?: string }> {
  switch (name) {
    case "list-todo":
      return ListTodo;
    case "compass":
      return Compass;
    case "calendar":
    case "calendar-range":
      return CalendarRange;
    case "eye":
      return Eye;
    default:
      return Sparkles;
  }
}

function toRecipe(r: UserRecipe): Recipe {
  return {
    label: r.label,
    prompt: r.prompt,
    icon: iconForName(r.icon),
    userDefined: true,
  };
}

const RECIPES: Recipe[] = [
  {
    label: "List recent todos",
    icon: ListTodo,
    prompt: "List my open action items across recent meetings, grouped by meeting.",
  },
  {
    label: "Coach me",
    icon: Compass,
    prompt:
      "Based on my recent meetings, coach me: what should I focus on next, and what am I doing well?",
  },
  {
    label: "Write weekly recap",
    icon: CalendarRange,
    prompt:
      "Write a recap of my meetings this week: key decisions, action items, and recurring themes.",
  },
  {
    label: "Streamline my calendar",
    icon: Sparkles,
    prompt:
      "Looking at my recent meetings, suggest how to streamline my calendar — what could be shorter, async, or dropped.",
  },
  {
    label: "Blind spots",
    icon: Eye,
    prompt:
      "What blind spots or unresolved questions show up across my recent meetings?",
  },
];

interface Msg {
  role: "user" | "assistant";
  content: string;
  coverage?: CoverageNote;
}

function CoveragePanel({ coverage }: { coverage: CoverageNote }) {
  const [open, setOpen] = React.useState(false);

  const parts: string[] = [];
  if (coverage.notes_read > 0) {
    const range =
      coverage.date_oldest &&
      coverage.date_newest &&
      coverage.date_oldest !== coverage.date_newest
        ? ` (${coverage.date_oldest} — ${coverage.date_newest})`
        : coverage.date_newest
          ? ` (${coverage.date_newest})`
          : "";
    parts.push(
      `${coverage.notes_read} of ${coverage.notes_total} note${coverage.notes_total === 1 ? "" : "s"}${range}`
    );
  } else {
    parts.push("no notes with summaries yet");
  }
  if (coverage.memories > 0) {
    parts.push(`${coverage.memories} memory${coverage.memories === 1 ? "y" : "ies"}`);
  }
  if (coverage.tasks > 0) {
    parts.push(`${coverage.tasks} open task${coverage.tasks === 1 ? "" : "s"}`);
  }

  return (
    <div className="mt-1">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex items-center gap-1 text-2xs text-muted-foreground/60 hover:text-muted-foreground"
        aria-expanded={open}
      >
        <span>{open ? "▾" : "▸"}</span>
        <span>Coverage</span>
      </button>
      {open ? (
        <div className="mt-1 rounded-md border border-border/50 bg-card/60 px-2.5 py-2 text-2xs text-muted-foreground">
          <p className="font-medium text-foreground/70">Searched</p>
          <ul className="mt-0.5 space-y-0.5 pl-3">
            {parts.map((p) => (
              <li key={p} className="list-disc">
                {p}
              </li>
            ))}
          </ul>
          {coverage.capped ? (
            <p className="mt-1.5 text-amber-600 dark:text-amber-400">
              Answer may be incomplete — only the {coverage.notes_read} most recent
              notes were read.
            </p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function formatRecentTime(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleDateString([], { month: "short", day: "numeric" });
}

export default function Chat() {
  const location = useLocation();
  const seed = (location.state as { seed?: string } | null)?.seed;
  const seededRef = React.useRef(false);

  const [messages, setMessages] = React.useState<Msg[]>([]);
  const [input, setInput] = React.useState("");
  const [busy, setBusy] = React.useState(false);
  const [models, setModels] = React.useState<ModelInfo[]>([]);
  const [model, setModel] = React.useState<string>("");
  const scrollRef = React.useRef<HTMLDivElement>(null);
  const inputRef = React.useRef<HTMLInputElement>(null);

  const [userRecipes, setUserRecipes] = React.useState<Recipe[]>([]);
  React.useEffect(() => {
    listRecipes()
      .then((rs) => setUserRecipes(rs.map(toRecipe)))
      .catch(() => {});
  }, []);

  const allRecipes = React.useMemo(() => [...RECIPES, ...userRecipes], [userRecipes]);

  const [paletteOpen, setPaletteOpen] = React.useState(false);
  const [paletteQuery, setPaletteQuery] = React.useState("");
  const [paletteIndex, setPaletteIndex] = React.useState(0);

  const paletteMatches = React.useMemo(() => {
    if (!paletteQuery) return allRecipes;
    const q = paletteQuery.toLowerCase();
    return allRecipes.filter((r) => r.label.toLowerCase().includes(q));
  }, [allRecipes, paletteQuery]);

  const threadIdRef = React.useRef<string | null>(null);
  const createdAtRef = React.useRef<string | null>(null);
  const [recents, setRecents] = React.useState<ChatThread[]>([]);
  const [recentsOpen, setRecentsOpen] = React.useState(false);

  const loadRecents = React.useCallback(() => {
    listChatThreads("library")
      .then(setRecents)
      .catch((e) => console.error("list_chat_threads:", e));
  }, []);
  React.useEffect(() => loadRecents(), [loadRecents]);

  React.useEffect(() => {
    let cancelled = false;
    void listProviderModels("openai")
      .then((ms) => {
        if (cancelled) return;
        setModels(ms);
        setModel((cur) => cur || ms[0]?.id || "");
      })
      .catch((e) => console.error("listProviderModels:", e));
    return () => {
      cancelled = true;
    };
  }, []);

  React.useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [messages, busy]);

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
        scope: "library",
        session_dir: null,
        title,
        created_at: createdAtRef.current ?? now,
        updated_at: now,
        messages: msgs.map((m) => ({ role: m.role, content: m.content })),
      };
      saveChatThread(thread)
        .then(loadRecents)
        .catch((e) => console.error("save_chat_thread:", e));
    },
    [loadRecents]
  );

  const closePalette = React.useCallback(() => {
    setPaletteOpen(false);
    setPaletteQuery("");
    setPaletteIndex(0);
  }, []);

  const ask = React.useCallback(
    async (question: string) => {
      const q = question.trim();
      if (!q || busy) return;
      closePalette();
      const history: ChatTurn[] = messages.map((m) => ({
        role: m.role,
        content: m.content,
      }));
      setMessages((prev) => [...prev, { role: "user", content: q }]);
      setInput("");
      setBusy(true);

      try {
        const { answer, coverage } = await askLibrary(q, history, model || undefined);
        setMessages((prev) => {
          const next: Msg[] = [
            ...prev,
            { role: "assistant", content: answer, coverage },
          ];
          persist(next);
          return next;
        });
      } catch (e) {
        console.error("ask_library:", e);
        toast.error("Couldn't answer that", { description: humanizeError(e) });
        setMessages((prev) => [
          ...prev,
          { role: "assistant", content: "Sorry — I couldn't answer that just now." },
        ]);
      } finally {
        setBusy(false);
      }
    },
    [busy, closePalette, messages, model, persist]
  );

  const openThread = React.useCallback((t: ChatThread) => {
    threadIdRef.current = t.id;
    createdAtRef.current = t.created_at;
    setMessages(
      t.messages.map((m) => ({ role: m.role as Msg["role"], content: m.content }))
    );
    setRecentsOpen(false);
  }, []);

  const newChat = React.useCallback(() => {
    threadIdRef.current = null;
    createdAtRef.current = null;
    setMessages([]);
    setRecentsOpen(false);
  }, []);

  const removeThread = React.useCallback(
    async (id: string, title: string) => {
      const ok = await confirmDelete({
        title: "Delete this conversation?",
        description: `"${title}" and its messages will be removed. This can't be undone.`,
        confirmLabel: "Delete conversation",
      });
      if (!ok) return;
      try {
        await deleteChatThread(id);
        if (threadIdRef.current === id) newChat();
        loadRecents();
      } catch (e) {
        console.error("delete_chat_thread:", e);
      }
    },
    [loadRecents, newChat]
  );

  React.useEffect(() => {
    if (seed && !seededRef.current) {
      seededRef.current = true;
      void ask(seed);
    }
  }, [seed, ask]);

  const empty = messages.length === 0 && !busy;

  return (
    <div className="mx-auto flex h-full w-full max-w-3xl flex-col px-8 py-8">
      <header data-drag="" className="flex select-none items-center justify-between">
        <h1 className="font-serif text-3xl font-medium tracking-tight">Ask anything</h1>
        <div className="flex items-center gap-1.5">
          <Button
            variant="ghost"
            size="sm"
            className="gap-1.5"
            onClick={newChat}
            title="New conversation"
          >
            <Plus className="h-3.5 w-3.5" />
            New
          </Button>
          <div className="relative">
            <Button
              variant="ghost"
              size="sm"
              aria-haspopup="menu"
              aria-expanded={recentsOpen}
              onClick={() => setRecentsOpen((v) => !v)}
            >
              Recents
            </Button>
            {recentsOpen ? (
              <>
                <button
                  type="button"
                  aria-hidden="true"
                  tabIndex={-1}
                  className="fixed inset-0 z-10 cursor-default"
                  onClick={() => setRecentsOpen(false)}
                />
                <div
                  role="menu"
                  className="absolute right-0 top-full z-20 mt-1 max-h-80 w-80 overflow-y-auto rounded-md border border-border bg-popover py-1 text-sm shadow-lg"
                >
                  {recents.length === 0 ? (
                    <p className="px-3 py-3 text-xs text-muted-foreground">
                      No conversations yet.
                    </p>
                  ) : (
                    recents.map((t) => (
                      <div
                        key={t.id}
                        className="group flex items-center gap-2 px-2 py-1.5 hover:bg-accent"
                      >
                        <button
                          type="button"
                          onClick={() => openThread(t)}
                          className="min-w-0 flex-1 text-left"
                        >
                          <p className="truncate text-foreground">{t.title}</p>
                          <p className="truncate text-2xs text-muted-foreground">
                            {formatRecentTime(t.updated_at)}
                          </p>
                        </button>
                        <button
                          type="button"
                          onClick={() => void removeThread(t.id, t.title)}
                          aria-label={`Delete conversation ${t.title}`}
                          className="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:text-destructive group-hover:opacity-100"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      </div>
                    ))
                  )}
                </div>
              </>
            ) : null}
          </div>

          {models.length > 0 ? (
            <select
              value={model}
              onChange={(e) => setModel(e.target.value)}
              aria-label="Model"
              className="h-8 rounded-md border border-input bg-card px-2 text-xs shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              {models.map((m) => (
                <option key={m.id} value={m.id}>
                  {m.display_name}
                </option>
              ))}
            </select>
          ) : null}
        </div>
      </header>

      <div ref={scrollRef} className="mt-6 flex-1 space-y-3 overflow-y-auto">
        {empty ? (
          <div className="flex flex-wrap gap-2">
            {allRecipes.map((r) => {
              const Icon = r.icon;
              return (
                <button
                  key={r.label}
                  type="button"
                  onClick={() => void ask(r.prompt)}
                  className="inline-flex items-center gap-2 rounded-full border border-border bg-card px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:text-foreground"
                >
                  <Icon className="h-3.5 w-3.5" />
                  {r.label}
                  {r.userDefined ? (
                    <span
                      className="h-1.5 w-1.5 rounded-full bg-primary/40"
                      title="Your recipe"
                    />
                  ) : null}
                </button>
              );
            })}
          </div>
        ) : null}

        {messages.map((m, i) => (
          <div
            key={i}
            className={
              m.role === "user" ? "ml-auto max-w-[85%]" : "mr-auto max-w-[90%]"
            }
          >
            <div
              className={
                m.role === "user"
                  ? "whitespace-pre-wrap rounded-lg bg-primary px-3 py-2 text-sm text-primary-foreground"
                  : "rounded-lg bg-muted px-3 py-2 text-sm leading-relaxed"
              }
            >
              {m.role === "assistant" ? <Markdown>{m.content}</Markdown> : m.content}
            </div>
            {m.role === "assistant" && m.coverage ? (
              <CoveragePanel coverage={m.coverage} />
            ) : null}
          </div>
        ))}
        {busy ? (
          <div className="mr-auto flex items-center gap-2 rounded-lg bg-muted px-3 py-2 text-sm text-muted-foreground">
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
            Thinking…
          </div>
        ) : null}
      </div>

      {paletteOpen && paletteMatches.length > 0 ? (
        <div
          role="listbox"
          aria-label="Recipe palette"
          className="mt-2 overflow-hidden rounded-lg border border-border bg-popover shadow-md"
        >
          {paletteQuery === "" ? (
            <p className="px-3 pt-2 text-2xs text-muted-foreground">
              Recipes — type to filter, ↑↓ to navigate, ↵ to run
            </p>
          ) : null}
          {paletteMatches.map((r, i) => {
            const Icon = r.icon;
            return (
              <button
                key={r.label}
                type="button"
                role="option"
                aria-selected={i === paletteIndex}
                onMouseEnter={() => setPaletteIndex(i)}
                onClick={() => {
                  setInput("");
                  void ask(r.prompt);
                }}
                className={cn(
                  "flex w-full items-center gap-2.5 px-3 py-2 text-left text-sm",
                  i === paletteIndex
                    ? "bg-accent text-accent-foreground"
                    : "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
                )}
              >
                <Icon className="h-3.5 w-3.5 shrink-0" />
                <span className="font-medium">{r.label}</span>
              </button>
            );
          })}
        </div>
      ) : null}

      <form
        className="mt-2 flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (paletteOpen && paletteMatches.length > 0) {
            const recipe = paletteMatches[paletteIndex] ?? paletteMatches[0];
            if (recipe) {
              setInput("");
              void ask(recipe.prompt);
            }
            return;
          }
          void ask(input);
        }}
      >
        <input
          ref={inputRef}
          value={input}
          onChange={(e) => {
            const val = e.target.value;
            setInput(val);
            if (val.startsWith("/")) {
              setPaletteOpen(true);
              setPaletteQuery(val.slice(1));
              setPaletteIndex(0);
            } else {
              setPaletteOpen(false);
              setPaletteQuery("");
            }
          }}
          onKeyDown={(e) => {
            if (!paletteOpen) return;
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setPaletteIndex((i) => Math.min(i + 1, paletteMatches.length - 1));
            } else if (e.key === "ArrowUp") {
              e.preventDefault();
              setPaletteIndex((i) => Math.max(i - 1, 0));
            } else if (e.key === "Escape") {
              e.preventDefault();
              closePalette();
              setInput("");
            }
          }}
          placeholder="Ask across all your meetings… (/ for recipes)"
          aria-label="Ask across your library"
          aria-autocomplete="list"
          aria-controls={paletteOpen ? "recipe-palette" : undefined}
          className="h-11 flex-1 rounded-lg border border-input bg-card px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
        <Button
          type="submit"
          size="lg"
          className="gap-2"
          disabled={busy || (!input.trim() && !paletteOpen)}
        >
          <Send className="h-4 w-4" />
          Ask
        </Button>
      </form>
    </div>
  );
}
