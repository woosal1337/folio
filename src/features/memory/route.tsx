/**
 * /memory — the user-facing surface for the local memory layer.
 *
 * Cards grouped by kind (All / Identity / Preferences / People /
 * Observations) with a search bar at the top, an archived toggle in
 * the header, and per-card edit / pin / archive controls. New memories
 * appear automatically after each recording when the auto-extract
 * toggle is on; the user can also add their own from the inline
 * composer (which always uses the Observe kind — for keyed memories,
 * the user clicks "Add memory" and picks the kind in the edit modal).
 */

import * as React from "react";
import {
  ArrowUpRight,
  Brain,
  Copy,
  ExternalLink,
  Loader2,
  Pin,
  PinOff,
  Plus,
  RefreshCw,
  Sparkles,
  Trash2,
  X,
} from "lucide-react";
import { Link } from "react-router-dom";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { cn } from "@/shared/lib/utils";
import { memoryFilePath } from "@/shared/lib/ipc";
import { copyToClipboard, memoryToMarkdown, openInObsidian } from "@/shared/lib/share";
import { useMemoriesStore } from "@/shared/stores/memories-store";
import type { Memory } from "@/shared/types/Memory";
import type { MemoryKind } from "@/shared/types/MemoryKind";
import type { MemoryUpdate } from "@/shared/types/MemoryUpdate";
import type { NewMemory } from "@/shared/types/NewMemory";

const KIND_META: Record<
  MemoryKind,
  { label: string; description: string; accent: string }
> = {
  observe: {
    label: "Observations",
    description: "Free-form context",
    accent: "text-slate-500",
  },
  claim: {
    label: "Claims",
    description: "Facts about you & your projects",
    accent: "text-primary",
  },
  pref: {
    label: "Preferences",
    description: "How you like things",
    accent: "text-amber-600",
  },
  person: {
    label: "People",
    description: "Who you work with",
    accent: "text-emerald-600",
  },
};

const KIND_ORDER: MemoryKind[] = ["claim", "pref", "person", "observe"];

const blankNewMemory = (kind: MemoryKind, content: string): NewMemory => ({
  kind,
  key: null,
  content,
  evidence: null,
  confidence: 1.0,
  tags: [],
  source_session_dir: null,
  source_session_label: null,
});

const blankUpdate = (): MemoryUpdate => ({
  content: null,
  key: null,
  evidence: null,
  tags: null,
  pinned: null,
});

export default function MemoryRoute() {
  const memories = useMemoriesStore((s) => s.memories);
  const loading = useMemoriesStore((s) => s.loading);
  const includeArchived = useMemoriesStore((s) => s.includeArchived);
  const kindsFilter = useMemoriesStore((s) => s.kindsFilter);
  const refresh = useMemoriesStore((s) => s.refresh);
  const create = useMemoriesStore((s) => s.create);
  const update = useMemoriesStore((s) => s.update);
  const pin = useMemoriesStore((s) => s.pin);
  const remove = useMemoriesStore((s) => s.remove);
  const purge = useMemoriesStore((s) => s.purge);
  const rebuildIndex = useMemoriesStore((s) => s.rebuildIndex);
  const setIncludeArchived = useMemoriesStore((s) => s.setIncludeArchived);
  const setKindsFilter = useMemoriesStore((s) => s.setKindsFilter);

  const [editing, setEditing] = React.useState<Memory | null>(null);
  const [searchQ, setSearchQ] = React.useState("");

  React.useEffect(() => {
    refresh();
  }, [refresh]);

  const activeKind = kindsFilter[0] ?? null;
  const setActiveKind = (kind: MemoryKind | null) => {
    setKindsFilter(kind ? [kind] : []);
  };

  const filtered = React.useMemo(() => {
    const lower = searchQ.trim().toLowerCase();
    if (!lower) return memories;
    return memories.filter(
      (m) =>
        m.content.toLowerCase().includes(lower) ||
        (m.key ?? "").toLowerCase().includes(lower) ||
        m.tags.some((t) => t.toLowerCase().includes(lower))
    );
  }, [memories, searchQ]);

  const current = filtered.filter((m) => m.valid_until === null);
  const archived = filtered.filter((m) => m.valid_until !== null);

  return (
    <div className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-8 py-8">
      <header data-drag="" className="flex select-none items-end justify-between gap-4">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">Memory</h1>
          <p className="mt-1 max-w-2xl text-sm text-muted-foreground">
            What Attune remembers about you across recordings. Facts the Extract
            Memories agent picks up after each meeting land here automatically and get
            used as background context the next time you run any agent.
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">
            {current.length === 0
              ? "No memories yet"
              : `${current.length} ${current.length === 1 ? "memory" : "memories"}`}
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={async () => {
              const n = await rebuildIndex();
              if (n !== null) {
                toast.success(`Reindexed ${n} ${n === 1 ? "memory" : "memories"}`);
              }
            }}
            title="Rebuild the FTS5 + vec index from the markdown files"
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Reindex
          </Button>
        </div>
      </header>

      {/* Filter row */}
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={() => setActiveKind(null)}
          aria-pressed={activeKind === null}
          className={cn(
            "rounded-md border px-3 py-1 text-xs font-medium transition-colors",
            activeKind === null
              ? "border-primary bg-accent text-accent-foreground"
              : "border-border bg-card text-muted-foreground hover:bg-secondary"
          )}
        >
          All
        </button>
        {KIND_ORDER.map((k) => (
          <button
            type="button"
            key={k}
            onClick={() => setActiveKind(activeKind === k ? null : k)}
            aria-pressed={activeKind === k}
            className={cn(
              "rounded-md border px-3 py-1 text-xs font-medium transition-colors",
              activeKind === k
                ? "border-primary bg-accent text-accent-foreground"
                : "border-border bg-card text-muted-foreground hover:bg-secondary"
            )}
          >
            {KIND_META[k].label}
          </button>
        ))}
        <div className="ml-auto flex items-center gap-2">
          <Input
            value={searchQ}
            onChange={(e) => setSearchQ(e.target.value)}
            placeholder="Search content, keys, tags…"
            className="h-8 w-56"
          />
          <label className="flex cursor-pointer items-center gap-1 text-xs text-muted-foreground">
            <input
              type="checkbox"
              checked={includeArchived}
              onChange={(e) => setIncludeArchived(e.target.checked)}
              className="h-3 w-3"
            />
            Include archived
          </label>
        </div>
      </div>

      {loading && memories.length === 0 ? (
        <div className="flex flex-1 items-center justify-center py-24 text-muted-foreground">
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          Loading…
        </div>
      ) : null}

      {!loading && memories.length === 0 ? (
        <EmptyState
          onCreate={(content) => create(blankNewMemory("observe", content))}
        />
      ) : null}

      {current.length > 0 ? (
        <section className="space-y-2">
          <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
            {current.map((m) => (
              <MemoryCard
                key={m.id}
                memory={m}
                onOpen={() => setEditing(m)}
                onPin={(pinned) => pin(m.id, pinned)}
                onArchive={() => remove(m.id)}
              />
            ))}
          </div>
        </section>
      ) : null}

      {/* Inline composer for free-form Observe */}
      <InlineComposer
        onCreate={(content) => create(blankNewMemory("observe", content))}
      />

      {includeArchived && archived.length > 0 ? (
        <section className="space-y-2 pt-6">
          <h2 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Archived
          </h2>
          <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
            {archived.map((m) => (
              <MemoryCard
                key={m.id}
                memory={m}
                onOpen={() => setEditing(m)}
                onPin={(pinned) => pin(m.id, pinned)}
                onArchive={async () => {
                  if (window.confirm(`Permanently delete "${m.content}"?`)) {
                    await purge(m.id);
                  }
                }}
                archivedLook
              />
            ))}
          </div>
        </section>
      ) : null}

      <EditMemoryDialog
        memory={editing}
        onClose={() => setEditing(null)}
        onSave={async (patch) => {
          if (!editing) return;
          await update(editing.id, patch);
          setEditing(null);
        }}
      />
    </div>
  );
}

interface MemoryCardProps {
  memory: Memory;
  onOpen: () => void;
  onPin: (pinned: boolean) => void;
  onArchive: () => void;
  archivedLook?: boolean;
}

function MemoryCard({
  memory,
  onOpen,
  onPin,
  onArchive,
  archivedLook,
}: MemoryCardProps) {
  const meta = KIND_META[memory.kind];
  return (
    <article
      className={cn(
        "group flex flex-col gap-2 rounded-lg border border-border bg-card p-3 text-left shadow-sm transition-shadow hover:shadow-md",
        archivedLook && "opacity-60"
      )}
    >
      <header className="flex items-start justify-between gap-2">
        <div className="flex min-w-0 flex-col gap-0.5">
          <div className="flex items-center gap-1.5">
            <Brain className={cn("h-3.5 w-3.5", meta.accent)} />
            <span className="text-2xs uppercase tracking-wider text-muted-foreground">
              {memory.kind}
            </span>
            {memory.key && (
              <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-2xs">
                {memory.key}
              </code>
            )}
            {memory.pinned && (
              <Pin className="h-3 w-3 text-amber-600" aria-label="pinned" />
            )}
          </div>
          <button
            type="button"
            onClick={onOpen}
            className="mt-1 text-left text-sm leading-snug"
          >
            {memory.content}
          </button>
        </div>
        <div className="flex shrink-0 items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onPin(!memory.pinned);
            }}
            className="rounded p-1 text-muted-foreground hover:bg-secondary hover:text-foreground"
            aria-label={memory.pinned ? "Unpin" : "Pin"}
            title={
              memory.pinned ? "Unpin (drop from always-inject)" : "Pin (always inject)"
            }
          >
            {memory.pinned ? (
              <PinOff className="h-3.5 w-3.5" />
            ) : (
              <Pin className="h-3.5 w-3.5" />
            )}
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onArchive();
            }}
            className="rounded p-1 text-muted-foreground hover:bg-secondary hover:text-foreground"
            aria-label={archivedLook ? "Delete permanently" : "Archive"}
            title={archivedLook ? "Delete permanently" : "Archive (soft-delete)"}
          >
            <Trash2 className="h-3.5 w-3.5" />
          </button>
        </div>
      </header>
      {memory.evidence && (
        <blockquote className="border-l-2 border-border pl-2 text-2xs italic text-muted-foreground">
          {memory.evidence}
        </blockquote>
      )}
      {memory.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {memory.tags.map((t) => (
            <Badge key={t} variant="outline" className="text-2xs">
              {t}
            </Badge>
          ))}
        </div>
      )}
      <footer className="mt-1 flex items-center justify-between gap-2 text-2xs text-muted-foreground">
        <div className="flex items-center gap-1">
          {memory.confidence < 0.7 && (
            <span title={`Confidence ${(memory.confidence * 100).toFixed(0)}%`}>
              ◔ {Math.round(memory.confidence * 100)}%
            </span>
          )}
          {memory.supersedes_id && (
            <span title="Supersedes a prior memory">↻ updated</span>
          )}
        </div>
        {memory.source_session_label ? (
          <Link
            to={`/editor/${memory.source_session_label}`}
            onClick={(e) => e.stopPropagation()}
            className="inline-flex items-center gap-0.5 truncate hover:text-foreground hover:underline"
            title={memory.source_session_label}
          >
            <Sparkles className="h-3 w-3" />
            <span className="max-w-[140px] truncate">
              {memory.source_session_label}
            </span>
            <ArrowUpRight className="h-3 w-3 shrink-0" />
          </Link>
        ) : (
          <span className="text-muted-foreground/60">manual</span>
        )}
      </footer>
    </article>
  );
}

interface InlineComposerProps {
  onCreate: (content: string) => void;
}

function InlineComposer({ onCreate }: InlineComposerProps) {
  const [open, setOpen] = React.useState(false);
  const [value, setValue] = React.useState("");
  const ref = React.useRef<HTMLTextAreaElement | null>(null);
  React.useEffect(() => {
    if (open && ref.current) ref.current.focus();
  }, [open]);

  const submit = () => {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      setOpen(false);
      return;
    }
    onCreate(trimmed);
    setValue("");
  };

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="mt-2 inline-flex w-full items-center justify-center gap-1.5 rounded-lg border border-dashed border-border bg-transparent px-3 py-3 text-xs text-muted-foreground transition-colors hover:border-primary hover:bg-card hover:text-foreground"
      >
        <Plus className="h-3.5 w-3.5" />
        Add an observation
      </button>
    );
  }

  return (
    <div className="mt-2 flex flex-col gap-1.5 rounded-lg border border-border bg-card p-2">
      <textarea
        ref={ref}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            submit();
          } else if (e.key === "Escape") {
            e.preventDefault();
            setValue("");
            setOpen(false);
          }
        }}
        rows={2}
        placeholder="Something to remember…"
        className="resize-none rounded border border-input bg-background px-2 py-1.5 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      />
      <div className="flex items-center justify-between text-2xs text-muted-foreground">
        <span>↵ to add · esc to close — saved as an observation</span>
        <button
          type="button"
          onClick={() => {
            setValue("");
            setOpen(false);
          }}
          className="rounded p-0.5 hover:bg-secondary"
          aria-label="Close composer"
        >
          <X className="h-3 w-3" />
        </button>
      </div>
    </div>
  );
}

interface EditMemoryDialogProps {
  memory: Memory | null;
  onClose: () => void;
  onSave: (patch: MemoryUpdate) => Promise<void>;
}

function EditMemoryDialog({ memory, onClose, onSave }: EditMemoryDialogProps) {
  const [content, setContent] = React.useState("");
  const [key, setKey] = React.useState("");
  const [evidence, setEvidence] = React.useState("");
  const [tags, setTags] = React.useState("");
  const [saving, setSaving] = React.useState(false);

  React.useEffect(() => {
    if (!memory) return;
    setContent(memory.content);
    setKey(memory.key ?? "");
    setEvidence(memory.evidence ?? "");
    setTags(memory.tags.join(", "));
    setSaving(false);
  }, [memory]);

  const submit = async () => {
    if (!memory) return;
    setSaving(true);
    const patch: MemoryUpdate = {
      ...blankUpdate(),
      content: content.trim() || memory.content,
      key,
      evidence,
      tags: tags
        .split(",")
        .map((t) => t.trim())
        .filter(Boolean),
    };
    await onSave(patch);
  };

  return (
    <Dialog open={!!memory} onOpenChange={(open) => (!open ? onClose() : null)}>
      <DialogContent className="max-w-[560px] p-6">
        <DialogHeader>
          <DialogTitle>
            Edit memory{" "}
            <span className="text-xs text-muted-foreground">({memory?.kind})</span>
          </DialogTitle>
        </DialogHeader>
        <div className="grid gap-4">
          <div className="grid gap-1.5">
            <Label htmlFor="m-content">Content</Label>
            <textarea
              id="m-content"
              value={content}
              onChange={(e) => setContent(e.target.value)}
              rows={3}
              className="resize-none rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1.5">
              <Label htmlFor="m-key">Key</Label>
              <Input
                id="m-key"
                value={key}
                onChange={(e) => setKey(e.target.value)}
                placeholder="user.company"
                className="font-mono text-xs"
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="m-tags">Tags (comma-separated)</Label>
              <Input
                id="m-tags"
                value={tags}
                onChange={(e) => setTags(e.target.value)}
                placeholder="identity, company"
              />
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="m-evidence">Evidence (optional)</Label>
            <textarea
              id="m-evidence"
              value={evidence}
              onChange={(e) => setEvidence(e.target.value)}
              rows={2}
              placeholder="Quoted snippet from the transcript"
              className="resize-none rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            />
          </div>
          {memory?.source_session_label && (
            <div className="rounded-md bg-secondary/50 px-3 py-2 text-xs text-muted-foreground">
              From{" "}
              <Link
                to={`/editor/${memory.source_session_label}`}
                className="font-medium text-foreground hover:underline"
                onClick={onClose}
              >
                {memory.source_session_label}
              </Link>
            </div>
          )}
        </div>
        <DialogFooter className="sm:justify-between">
          <div className="flex items-center gap-1">
            {memory ? (
              <>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    memory &&
                    copyToClipboard(memoryToMarkdown(memory), "Markdown copied")
                  }
                  title="Copy as Markdown"
                >
                  <Copy className="mr-1.5 h-3.5 w-3.5" />
                  Copy MD
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={async () => {
                    if (!memory) return;
                    const path = await memoryFilePath(memory.id).catch(() => null);
                    await openInObsidian(path);
                  }}
                  title="Open the source file in Obsidian"
                >
                  <ExternalLink className="mr-1.5 h-3.5 w-3.5" />
                  Open in Obsidian
                </Button>
              </>
            ) : null}
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" onClick={onClose} disabled={saving}>
              Cancel
            </Button>
            <Button onClick={submit} disabled={saving || content.trim().length === 0}>
              {saving ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              Save
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface EmptyStateProps {
  onCreate: (content: string) => void;
}

function EmptyState({ onCreate }: EmptyStateProps) {
  return (
    <div className="rounded-2xl border border-dashed border-border bg-card py-16">
      <div className="mx-auto flex max-w-md flex-col items-center gap-3 text-center">
        <Brain className="h-8 w-8 text-muted-foreground" />
        <h2 className="font-medium">Nothing remembered yet</h2>
        <p className="text-sm text-muted-foreground">
          Attune will start populating this page after your next recording, assuming
          auto-extract memories is on in Settings → AI. You can also add an observation
          yourself.
        </p>
        <div className="mt-2 w-full max-w-xs">
          <InlineComposer onCreate={onCreate} />
        </div>
      </div>
    </div>
  );
}
