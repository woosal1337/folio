import * as React from "react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import {
  FileAudio,
  FileText,
  Loader2,
  Lock,
  Mic,
  MoreHorizontal,
  Plus,
  RefreshCw,
  Sparkles,
} from "lucide-react";

import { AskBar } from "@/chrome/ask-bar";
import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { SyncBadge } from "@/shared/ui/sync-badge";
import { useNoteContextMenu } from "@/shared/hooks/use-note-context-menu";
import { useQuickNote, useTakeNotes } from "@/shared/hooks/use-take-notes";
import { humanizeError } from "@/shared/lib/errors";
import { listRecordings, searchNoteContent } from "@/shared/lib/ipc";
import { useJobsStore } from "@/shared/stores/jobs-store";
import { useRecording } from "@/shared/stores/recording-store";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

import { NoteFilters, type SortOrder, type TranscriptFilter } from "./note-filters";

type Group = "Today" | "Yesterday" | "Earlier";

const GROUP_ORDER: Group[] = ["Today", "Yesterday", "Earlier"];

function groupFor(createdAt: string | null): Group {
  if (!createdAt) return "Earlier";
  const d = new Date(createdAt);
  if (Number.isNaN(d.getTime())) return "Earlier";
  const startOfToday = new Date();
  startOfToday.setHours(0, 0, 0, 0);
  const startOfYesterday = new Date(startOfToday);
  startOfYesterday.setDate(startOfYesterday.getDate() - 1);
  if (d >= startOfToday) return "Today";
  if (d >= startOfYesterday) return "Yesterday";
  return "Earlier";
}

function stampFor(createdAt: string | null, group: Group): string {
  if (!createdAt) return "";
  const d = new Date(createdAt);
  if (Number.isNaN(d.getTime())) return "";
  return group === "Earlier"
    ? d.toLocaleDateString([], { month: "short", day: "numeric" })
    : d.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

function titleFor(item: RecordingSummary): string {
  return (
    item.title?.trim() || item.suggested_title?.trim() || item.draft_name || item.label
  );
}

export default function Home() {
  const navigate = useNavigate();
  const takeNotes = useTakeNotes();
  const quickNote = useQuickNote();

  const transcribingDir = useRecording((s) => s.transcribingDir);
  const lastSavedDir = useRecording((s) => s.lastSavedDir);
  const lastTranscriptPath = useRecording((s) => s.lastTranscriptPath);

  const [recordings, setRecordings] = React.useState<RecordingSummary[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [query, setQuery] = React.useState("");
  const [filter, setFilter] = React.useState<TranscriptFilter>("all");
  const [sort, setSort] = React.useState<SortOrder>("newest");

  const refresh = React.useCallback(async () => {
    try {
      setRecordings(await listRecordings());
    } catch (e) {
      console.error("list_recordings:", e);
      toast.error("Could not load notes", { description: humanizeError(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    void refresh();
  }, [refresh, lastSavedDir, lastTranscriptPath]);

  const jobCount = useJobsStore((s) => Object.keys(s.jobs).length);
  const prevJobCount = React.useRef(jobCount);
  React.useEffect(() => {
    if (jobCount < prevJobCount.current) void refresh();
    prevJobCount.current = jobCount;
  }, [jobCount, refresh]);

  const openContextMenu = useNoteContextMenu(refresh);

  const open = React.useCallback(
    (item: RecordingSummary) => {
      navigate(`/editor/${encodeURIComponent(item.label)}`, {
        state: { recording: item },
      });
    },
    [navigate]
  );

  const [contentHits, setContentHits] = React.useState<Map<string, string>>(
    () => new Map()
  );
  React.useEffect(() => {
    const needle = query.trim();
    if (needle.length < 2) {
      setContentHits(new Map());
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      searchNoteContent(needle)
        .then((hits) => {
          if (cancelled) return;
          setContentHits(new Map(hits.map((h) => [h.session_dir, h.snippet])));
        })
        .catch((e) => console.error("search_note_content:", e));
    }, 150);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [query]);

  const visible = React.useMemo(() => {
    const needle = query.trim().toLowerCase();
    const out = recordings.filter((r) => {
      if (filter === "transcribed" && !r.has_transcript) return false;
      if (filter === "untranscribed" && r.has_transcript) return false;
      if (needle) {
        const hay =
          `${r.label} ${r.suggested_title ?? ""} ${r.title ?? ""}`.toLowerCase();
        if (!hay.includes(needle) && !contentHits.has(r.session_dir)) return false;
      }
      return true;
    });
    out.sort((a, b) => {
      const aTime = a.created_at ? Date.parse(a.created_at) : NaN;
      const bTime = b.created_at ? Date.parse(b.created_at) : NaN;
      const aOk = !Number.isNaN(aTime);
      const bOk = !Number.isNaN(bTime);
      if (aOk && bOk) return sort === "newest" ? bTime - aTime : aTime - bTime;
      if (aOk) return -1;
      if (bOk) return 1;
      return sort === "newest"
        ? b.label.localeCompare(a.label)
        : a.label.localeCompare(b.label);
    });
    return out;
  }, [recordings, query, filter, sort, contentHits]);

  const groups = React.useMemo(() => {
    const buckets: Record<Group, RecordingSummary[]> = {
      Today: [],
      Yesterday: [],
      Earlier: [],
    };
    for (const r of visible) buckets[groupFor(r.created_at)].push(r);
    const order = sort === "newest" ? GROUP_ORDER : [...GROUP_ORDER].reverse();
    return order
      .map((group) => ({ group, items: buckets[group] }))
      .filter((b) => b.items.length > 0);
  }, [visible, sort]);

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 py-8">
      <header data-drag="" className="flex select-none items-baseline justify-between">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">Home</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {`Every note you've taken, searchable. Click one to open it.`}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Button className="gap-2" onClick={() => takeNotes()}>
            <Mic className="h-4 w-4" />
            Take notes
          </Button>
          <Button variant="outline" className="gap-2" onClick={quickNote}>
            <Plus className="h-4 w-4" />
            Quick note
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => void refresh()}
            aria-label="Refresh notes"
            title="Refresh notes"
            className="text-muted-foreground hover:text-foreground"
          >
            <RefreshCw className="h-3.5 w-3.5" />
          </Button>
        </div>
      </header>

      <NoteFilters
        query={query}
        onQueryChange={setQuery}
        filter={filter}
        onFilterChange={setFilter}
        sort={sort}
        onSortChange={setSort}
      />

      {loading && recordings.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-sm text-muted-foreground">
            Loading…
          </CardContent>
        </Card>
      ) : recordings.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center gap-3 py-16 text-center">
            <div className="rounded-full border border-border bg-muted/40 p-3">
              <FileAudio className="h-6 w-6 text-muted-foreground" />
            </div>
            <h2 className="font-serif text-lg font-medium">
              Your notes will land here
            </h2>
            <p className="max-w-sm text-sm text-muted-foreground">
              Every note keeps its own transcript, summary, and audio in a folder under{" "}
              <code className="rounded bg-muted px-1 py-0.5 text-2xs">
                ~/Documents/Folio/Recordings/
              </code>{" "}
              — yours to keep.
            </p>
            <Button onClick={quickNote} className="mt-2 gap-1.5">
              Take your first note
            </Button>
          </CardContent>
        </Card>
      ) : groups.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-sm text-muted-foreground">
            No matches. Try clearing the search or switching the filter.
          </CardContent>
        </Card>
      ) : (
        groups.map(({ group, items }) => (
          <section key={group} className="space-y-1.5">
            <p className="px-1 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
              {group}
            </p>
            <div className="flex flex-col gap-1.5">
              {items.map((item) => (
                <NoteRow
                  key={item.session_dir}
                  item={item}
                  group={group}
                  transcribing={transcribingDir === item.session_dir}
                  snippet={contentHits.get(item.session_dir) ?? null}
                  onOpen={() => open(item)}
                  onMenu={(e) => openContextMenu(item, e)}
                />
              ))}
            </div>
          </section>
        ))
      )}

      <div className="mt-auto" />
      <AskBar />
    </div>
  );
}

function NoteRow({
  item,
  group,
  transcribing,
  snippet,
  onOpen,
  onMenu,
}: {
  item: RecordingSummary;
  group: Group;
  transcribing: boolean;
  snippet: string | null;
  onOpen: () => void;
  onMenu: (e: React.MouseEvent) => void;
}) {
  const secondary = snippet ?? item.suggested_subtitle ?? null;
  const synced = item.sync && item.sync.remote_status !== "none" ? item.sync : null;

  return (
    // eslint-disable-next-line jsx-a11y/no-static-element-interactions -- right-click affordance; every action is also reachable via the ⋯ button
    <div
      onContextMenu={onMenu}
      className="group flex items-center gap-3 rounded-lg border border-border bg-card px-4 py-3 transition-colors hover:bg-muted/40"
    >
      <button
        type="button"
        onClick={onOpen}
        className="flex min-w-0 flex-1 items-center gap-3 text-left"
      >
        <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{titleFor(item)}</p>
          {secondary ? (
            <p className="truncate text-xs text-muted-foreground">{secondary}</p>
          ) : null}
        </div>
      </button>

      {transcribing ? (
        <span className="inline-flex shrink-0 items-center gap-1 text-2xs text-muted-foreground">
          <Loader2 className="h-3 w-3 animate-spin" />
          Transcribing
        </span>
      ) : item.has_transcript ? (
        <span className="inline-flex shrink-0 items-center gap-1 text-2xs text-emerald-600 dark:text-emerald-400">
          <Sparkles className="h-3 w-3" />
          Transcribed
        </span>
      ) : null}

      {synced ? (
        <SyncBadge sync={synced} />
      ) : (
        <Lock
          className="h-3 w-3 shrink-0 text-muted-foreground"
          aria-label="Stored only on this Mac"
        />
      )}

      <span className="shrink-0 font-mono text-2xs text-muted-foreground">
        {stampFor(item.created_at, group)}
      </span>

      <Button
        type="button"
        variant="ghost"
        size="sm"
        aria-label="More actions"
        aria-haspopup="menu"
        onClick={onMenu}
        className="shrink-0 opacity-60 group-hover:opacity-100"
      >
        <MoreHorizontal className="h-4 w-4" />
      </Button>
    </div>
  );
}
