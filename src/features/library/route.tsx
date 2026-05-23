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

  const visible = React.useMemo(() => {
    const needle = query.trim().toLowerCase();
    const out = recordings.filter((r) => {
      if (filter === "transcribed" && !r.has_transcript) return false;
      if (filter === "untranscribed" && r.has_transcript) return false;
      if (needle && !r.label.toLowerCase().includes(needle)) return false;
      return true;
    });
    out.sort((a, b) =>
      sort === "newest"
        ? b.label.localeCompare(a.label)
        : a.label.localeCompare(b.label)
    );
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
          <CardContent className="flex flex-col items-center gap-3 py-12 text-center">
            <FileAudio className="h-7 w-7 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">
              No recordings yet.{" "}
              <Link to="/record" className="underline underline-offset-2">
                Start a session
              </Link>{" "}
              to see it here.
            </p>
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
    </div>
  );
}
