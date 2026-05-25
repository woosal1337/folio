import * as React from "react";
import { FileAudio, RefreshCw } from "lucide-react";
import { Link, useNavigate } from "react-router-dom";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { RecordingRow } from "@/features/recording/recording-row";
import { useRecording } from "@/shared/stores/recording-store";
import { deleteRecording, listRecordings, revealInFinder } from "@/shared/lib/ipc";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

import {
  LibraryFilters,
  type SortOrder,
  type TranscriptFilter,
} from "./library-filters";
import { QuickLookSheet } from "./quick-look-sheet";
import { StatsStrip } from "./stats-strip";

export default function Library() {
  const navigate = useNavigate();
  const transcribingDir = useRecording((s) => s.transcribingDir);
  const lastSavedDir = useRecording((s) => s.lastSavedDir);
  const lastTranscriptPath = useRecording((s) => s.lastTranscriptPath);
  const transcribe = useRecording((s) => s.transcribe);

  const [recordings, setRecordings] = React.useState<RecordingSummary[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [expanded, setExpanded] = React.useState<string | null>(null);
  const [query, setQuery] = React.useState("");
  const [filter, setFilter] = React.useState<TranscriptFilter>("all");
  const [sort, setSort] = React.useState<SortOrder>("newest");
  /** v2 finding 012 / GET-46: when non-null, render the Quick Look
   * sheet for this recording. Driven by a per-row Eye button and a
   * global Space-key handler that opens the row currently containing
   * document.activeElement. */
  const [quickLook, setQuickLook] = React.useState<RecordingSummary | null>(null);

  const refresh = React.useCallback(async () => {
    setLoading(true);
    try {
      const list = await listRecordings();
      setRecordings(list);
    } catch (e) {
      console.error("list_recordings:", e);
      toast.error("Could not load recordings", { description: String(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  // Reload on first mount and whenever a recording was just saved or
  // its transcription completed, so the library list stays in sync if
  // the user is sitting on this page while recording happens elsewhere.
  React.useEffect(() => {
    refresh();
  }, [refresh, lastSavedDir, lastTranscriptPath]);

  // Global Space-key handler. When the user has tabbed/clicked onto
  // a Library row card (any focusable element inside the card carries
  // the row's session_dir via the data-quicklook-session attribute on
  // the Card root), pressing Space opens the Quick Look sheet for
  // that recording. We ignore presses inside text inputs so the
  // search field still receives spaces.
  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== " ") return;
      if (quickLook) return; // sheet owns Space while it's open
      const active = document.activeElement;
      if (
        active instanceof HTMLInputElement ||
        active instanceof HTMLTextAreaElement ||
        (active instanceof HTMLElement && active.isContentEditable)
      ) {
        return;
      }
      const card =
        active instanceof HTMLElement
          ? active.closest("[data-quicklook-session]")
          : null;
      if (!(card instanceof HTMLElement)) return;
      const sessionDir = card.dataset.quicklookSession;
      if (!sessionDir) return;
      const hit = recordings.find((r) => r.session_dir === sessionDir);
      if (!hit) return;
      e.preventDefault();
      setQuickLook(hit);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [recordings, quickLook]);

  const visible = React.useMemo(() => {
    const needle = query.trim().toLowerCase();
    const out = recordings.filter((r) => {
      if (filter === "transcribed" && !r.has_transcript) return false;
      if (filter === "untranscribed" && r.has_transcript) return false;
      if (needle && !r.label.toLowerCase().includes(needle)) return false;
      return true;
    });
    // Sort by filesystem creation time when available — labels are
    // not reliably chronological once imported / hand-named sessions
    // (e.g. "2026-05-23-mark-cuban-yahoo-trade") enter the library.
    // Falls back to label-descending when created_at is null.
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
  }, [recordings, query, filter, sort]);

  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-6 px-8 py-10">
      <header data-drag="" className="flex select-none items-baseline justify-between">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">Library</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Every recording, with quick access to playback and transcripts.
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={refresh} className="gap-2">
          <RefreshCw className="h-3.5 w-3.5" />
          Refresh
        </Button>
      </header>

      <StatsStrip recordings={recordings} />

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
              Your meetings will land here
            </h2>
            <p className="max-w-sm text-sm text-muted-foreground">
              Each recording stores its WAV, transcript, summary, and any agent runs in
              its own folder under{" "}
              <code className="rounded bg-muted px-1 py-0.5 text-2xs">
                ~/Documents/Attune/Recordings/
              </code>{" "}
              — yours to keep.
            </p>
            <Link
              to="/record"
              className="mt-2 inline-flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-opacity hover:opacity-90"
            >
              Start your first recording
            </Link>
          </CardContent>
        </Card>
      ) : visible.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-sm text-muted-foreground">
            No matches. Try clearing the search or switching the filter.
          </CardContent>
        </Card>
      ) : (
        <div className="flex flex-col gap-2">
          {visible.map((item) => (
            <RecordingRow
              key={item.session_dir}
              item={item}
              open={expanded === item.session_dir}
              transcribing={transcribingDir === item.session_dir}
              onToggle={() =>
                setExpanded((cur) =>
                  cur === item.session_dir ? null : item.session_dir
                )
              }
              onOpenInEditor={() =>
                navigate(`/editor/${encodeURIComponent(item.label)}`, {
                  state: { recording: item },
                })
              }
              onQuickLook={() => setQuickLook(item)}
              onTranscribe={() => {
                // Fire-and-forget; the store flips `transcribingDir`
                // for the spinner and toasts on success/failure.
                void transcribe(item.session_dir);
              }}
              onSummarize={() =>
                navigate(`/editor/${encodeURIComponent(item.label)}`, {
                  state: { recording: item, autoRun: "summarize" },
                })
              }
              onReveal={() => {
                revealInFinder(item.session_dir).catch((e) => {
                  console.error("reveal_in_finder:", e);
                  toast.error("Could not open Finder", {
                    description: String(e),
                  });
                });
              }}
              onDelete={async () => {
                const ok = window.confirm(
                  `Delete this recording?\n\n${item.label}\n\nThis removes the session folder and every file inside it. Cannot be undone.`
                );
                if (!ok) return;
                try {
                  await deleteRecording(item.session_dir);
                  if (expanded === item.session_dir) setExpanded(null);
                  refresh();
                  toast.success("Recording deleted", { description: item.label });
                } catch (e) {
                  console.error("delete_recording:", e);
                  toast.error("Could not delete recording", {
                    description: String(e),
                  });
                }
              }}
            />
          ))}
        </div>
      )}

      <QuickLookSheet
        recording={quickLook}
        onClose={() => setQuickLook(null)}
        onOpenInEditor={(r) => {
          setQuickLook(null);
          navigate(`/editor/${encodeURIComponent(r.label)}`, {
            state: { recording: r },
          });
        }}
        onReveal={(r) => {
          revealInFinder(r.session_dir).catch((e) => {
            console.error("reveal_in_finder:", e);
            toast.error("Could not open Finder", { description: String(e) });
          });
        }}
      />
    </div>
  );
}
