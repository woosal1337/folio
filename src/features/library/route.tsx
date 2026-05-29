import * as React from "react";
import {
  FileAudio,
  FileText,
  Loader2,
  MoreHorizontal,
  RefreshCw,
  Sparkles,
  Trash2,
} from "lucide-react";
import { useNavigate, useSearchParams } from "react-router-dom";
import { toast } from "sonner";
import { Folder, X } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { useQuickNote } from "@/shared/hooks/use-take-notes";
import { useRecording } from "@/shared/stores/recording-store";
import {
  clearRecordingArtifacts,
  deleteRecording,
  listRecordings,
  revealInFinder,
} from "@/shared/lib/ipc";
import { t } from "@/shared/lib/i18n";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

import {
  LibraryFilters,
  type SortOrder,
  type TranscriptFilter,
} from "./library-filters";

export default function Library() {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const folderFilter = searchParams.get("folder");
  const quickNote = useQuickNote();
  const transcribingDir = useRecording((s) => s.transcribingDir);
  const lastSavedDir = useRecording((s) => s.lastSavedDir);
  const lastTranscriptPath = useRecording((s) => s.lastTranscriptPath);
  const transcribe = useRecording((s) => s.transcribe);

  const [recordings, setRecordings] = React.useState<RecordingSummary[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [query, setQuery] = React.useState("");
  const [filter, setFilter] = React.useState<TranscriptFilter>("all");
  const [sort, setSort] = React.useState<SortOrder>("newest");

  const refresh = React.useCallback(async () => {
    setLoading(true);
    try {
      setRecordings(await listRecordings());
    } catch (e) {
      console.error("list_recordings:", e);
      toast.error(t("errors.recordings.load"), { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    refresh();
  }, [refresh, lastSavedDir, lastTranscriptPath]);

  const open = React.useCallback(
    (item: RecordingSummary) => {
      navigate(`/editor/${encodeURIComponent(item.label)}`, {
        state: { recording: item },
      });
    },
    [navigate]
  );

  const onReveal = React.useCallback((item: RecordingSummary) => {
    revealInFinder(item.session_dir).catch((e) => {
      console.error("reveal_in_finder:", e);
      toast.error(t("errors.recording.reveal"), { description: String(e) });
    });
  }, []);

  const onReTranscribe = React.useCallback(
    async (item: RecordingSummary) => {
      try {
        await clearRecordingArtifacts(item.session_dir);
        void transcribe(item.session_dir);
        toast.success("Re-transcribing", { description: item.label });
      } catch (e) {
        console.error("re-transcribe:", e);
        toast.error("Could not re-transcribe", { description: String(e) });
      }
    },
    [transcribe]
  );

  const onDelete = React.useCallback(
    async (item: RecordingSummary) => {
      const ok = window.confirm(
        `Delete this note?\n\n${item.title?.trim() || item.suggested_title?.trim() || item.label}\n\nThis removes the session folder and every file inside it. Cannot be undone.`
      );
      if (!ok) return;
      try {
        await deleteRecording(item.session_dir);
        refresh();
        toast.success("Note deleted", { description: item.label });
      } catch (e) {
        console.error("delete_recording:", e);
        toast.error(t("errors.recording.delete"), { description: String(e) });
      }
    },
    [refresh]
  );

  const visible = React.useMemo(() => {
    const needle = query.trim().toLowerCase();
    const out = recordings.filter((r) => {
      if (folderFilter && r.folder !== folderFilter) return false;
      if (filter === "transcribed" && !r.has_transcript) return false;
      if (filter === "untranscribed" && r.has_transcript) return false;
      if (needle) {
        const hay =
          `${r.label} ${r.suggested_title ?? ""} ${r.title ?? ""}`.toLowerCase();
        if (!hay.includes(needle)) return false;
      }
      return true;
    });
    const compareBy = (a: RecordingSummary, b: RecordingSummary) => {
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
    };
    out.sort(compareBy);
    return out;
  }, [recordings, query, filter, sort, folderFilter]);

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-8">
      <header data-drag="" className="flex select-none items-baseline justify-between">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">
            {folderFilter ?? "My Notes"}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {folderFilter
              ? "Notes filed in this folder."
              : "Every note, searchable. Click one to open it."}
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={refresh} className="gap-2">
          <RefreshCw className="h-3.5 w-3.5" />
          Refresh
        </Button>
      </header>

      {folderFilter ? (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span className="inline-flex items-center gap-1.5 rounded-full border border-border bg-card px-2.5 py-1">
            <Folder className="h-3 w-3" />
            {folderFilter}
          </span>
          <button
            type="button"
            onClick={() => setSearchParams({})}
            className="inline-flex items-center gap-1 rounded-full px-2 py-1 transition-colors hover:text-foreground"
          >
            <X className="h-3 w-3" />
            Clear folder
          </button>
        </div>
      ) : null}

      <LibraryFilters
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
                ~/Documents/Attune/Recordings/
              </code>{" "}
              — yours to keep.
            </p>
            <Button onClick={quickNote} className="mt-2 gap-1.5">
              Take your first note
            </Button>
          </CardContent>
        </Card>
      ) : visible.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-sm text-muted-foreground">
            No matches. Try clearing the search or switching the filter.
          </CardContent>
        </Card>
      ) : (
        <div className="flex flex-col gap-1.5">
          {visible.map((item) => (
            <NoteRow
              key={item.session_dir}
              item={item}
              transcribing={transcribingDir === item.session_dir}
              onOpen={() => open(item)}
              onReveal={() => onReveal(item)}
              onReTranscribe={() => void onReTranscribe(item)}
              onDelete={() => void onDelete(item)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function NoteRow({
  item,
  transcribing,
  onOpen,
  onReveal,
  onReTranscribe,
  onDelete,
}: {
  item: RecordingSummary;
  transcribing: boolean;
  onOpen: () => void;
  onReveal: () => void;
  onReTranscribe: () => void;
  onDelete: () => void;
}) {
  const title = item.title?.trim() || item.suggested_title?.trim() || item.label;
  return (
    <div className="group flex items-center gap-3 rounded-lg border border-border bg-card px-4 py-3 transition-colors hover:bg-muted/40">
      <button
        type="button"
        onClick={onOpen}
        className="flex min-w-0 flex-1 items-center gap-3 text-left"
      >
        <FileText className="h-4 w-4 shrink-0 text-muted-foreground" />
        <div className="min-w-0">
          <p className="truncate text-sm font-medium">{title}</p>
          {item.suggested_subtitle ? (
            <p className="truncate text-xs text-muted-foreground">
              {item.suggested_subtitle}
            </p>
          ) : null}
        </div>
      </button>

      {transcribing ? (
        <span className="inline-flex items-center gap-1 text-2xs text-muted-foreground">
          <Loader2 className="h-3 w-3 animate-spin" />
          transcribing
        </span>
      ) : item.has_transcript ? (
        <span className="inline-flex items-center gap-1 text-2xs text-emerald-600 dark:text-emerald-400">
          <Sparkles className="h-3 w-3" />
          transcribed
        </span>
      ) : null}

      <span className="shrink-0 font-mono text-2xs text-muted-foreground">
        {formatRowTime(item.created_at)}
      </span>

      <RowMenu
        hasTranscript={item.has_transcript}
        onReveal={onReveal}
        onReTranscribe={onReTranscribe}
        onDelete={onDelete}
      />
    </div>
  );
}

function RowMenu({
  hasTranscript,
  onReveal,
  onReTranscribe,
  onDelete,
}: {
  hasTranscript: boolean;
  onReveal: () => void;
  onReTranscribe: () => void;
  onDelete: () => void;
}) {
  const [open, setOpen] = React.useState(false);
  const pick = (fn: () => void) => () => {
    setOpen(false);
    fn();
  };
  return (
    <div className="relative shrink-0">
      <Button
        type="button"
        variant="ghost"
        size="sm"
        aria-label="More actions"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
        className="opacity-60 group-hover:opacity-100"
      >
        <MoreHorizontal className="h-4 w-4" />
      </Button>
      {open ? (
        <>
          <button
            type="button"
            aria-hidden="true"
            tabIndex={-1}
            className="fixed inset-0 z-10 cursor-default"
            onClick={() => setOpen(false)}
          />
          <div
            role="menu"
            className="absolute right-0 top-full z-20 mt-1 w-44 overflow-hidden rounded-md border border-border bg-popover py-1 text-sm shadow-lg"
          >
            <RowMenuItem onClick={pick(onReveal)}>Reveal in Finder</RowMenuItem>
            {hasTranscript ? (
              <RowMenuItem onClick={pick(onReTranscribe)}>Re-transcribe</RowMenuItem>
            ) : null}
            <RowMenuItem onClick={pick(onDelete)} destructive>
              <Trash2 className="h-3.5 w-3.5" />
              Delete
            </RowMenuItem>
          </div>
        </>
      ) : null}
    </div>
  );
}

function RowMenuItem({
  onClick,
  children,
  destructive,
}: {
  onClick: () => void;
  children: React.ReactNode;
  destructive?: boolean;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onClick}
      className={
        "flex w-full items-center gap-2 px-3 py-1.5 text-left transition-colors " +
        (destructive
          ? "text-destructive hover:bg-destructive/10"
          : "text-foreground hover:bg-accent hover:text-accent-foreground")
      }
    >
      {children}
    </button>
  );
}

function formatRowTime(createdAt: string | null): string {
  if (!createdAt) return "";
  const d = new Date(createdAt);
  if (Number.isNaN(d.getTime())) return "";
  return d.toLocaleDateString([], { month: "short", day: "numeric" });
}
