import * as React from "react";
import { FileAudio, Loader2, Mic, RefreshCw, Square } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { RecordingRow } from "@/features/recording/recording-row";
import { StatusPill } from "@/features/recording/status-pill";
import { useRecording } from "@/shared/stores/recording-store";
import { deleteRecording, listRecordings, revealInFinder } from "@/shared/lib/ipc";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";

export default function Record() {
  const rec = useRecording();
  const syncFromBackend = useRecording((s) => s.syncFromBackend);
  const [history, setHistory] = React.useState<RecordingSummary[]>([]);
  const [expanded, setExpanded] = React.useState<string | null>(null);

  const refresh = React.useCallback(async () => {
    try {
      const list = await listRecordings();
      setHistory(list);
    } catch (e) {
      console.error("list_recordings:", e);
    }
  }, []);

  // First-mount: sync the store with the backend in case a session is
  // already running (e.g. after a window reload mid-recording), and
  // refresh the library list.
  React.useEffect(() => {
    syncFromBackend();
    refresh();
  }, [syncFromBackend, refresh]);

  React.useEffect(() => {
    if (rec.lastSavedDir) {
      refresh();
      setExpanded(rec.lastSavedDir);
    }
  }, [rec.lastSavedDir, refresh]);

  // After an auto-transcription completes, re-list so the row flips
  // from "transcribing" to "transcribed" without the user having to
  // hit Refresh.
  React.useEffect(() => {
    if (rec.lastTranscriptPath) {
      refresh();
    }
  }, [rec.lastTranscriptPath, refresh]);

  const elapsedLabel = React.useMemo(() => {
    const m = Math.floor(rec.elapsed / 60);
    const s = rec.elapsed % 60;
    return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }, [rec.elapsed]);

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-10">
      <header data-drag="" className="flex select-none items-baseline justify-between">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">Record</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            Capture system audio and microphone independently.
          </p>
        </div>
        <StatusPill recording={rec.recording} label={elapsedLabel} />
      </header>

      <Card>
        <CardContent className="flex flex-col items-center gap-4 py-12">
          {rec.recording ? (
            <Button
              size="xl"
              variant="destructive"
              className="w-full max-w-md gap-3"
              onClick={rec.stop}
              disabled={rec.busy}
            >
              <Square className="h-5 w-5 fill-current" />
              Stop recording
            </Button>
          ) : (
            <Button
              size="xl"
              className="w-full max-w-md gap-3"
              onClick={rec.start}
              disabled={rec.busy}
            >
              <Mic className="h-5 w-5" />
              {rec.busy ? "Starting…" : "Start recording"}
            </Button>
          )}
          <p className="text-xs text-muted-foreground">
            {rec.recording
              ? `Capturing ${rec.channels.length > 0 ? rec.channels.join(" + ") : "audio"} · transcribes automatically when you stop`
              : rec.transcribing
                ? "Transcribing the last recording…"
                : "Mic + system audio in parallel · transcribes automatically when you stop"}
          </p>
          {rec.transcribing && (
            <div
              className="flex items-center gap-2 text-xs text-muted-foreground"
              role="status"
              aria-live="polite"
            >
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              <span>Sending audio to OpenAI Whisper…</span>
            </div>
          )}
          {rec.error && <p className="text-xs text-destructive">{rec.error}</p>}
        </CardContent>
      </Card>

      <div className="flex items-center justify-between">
        <h2 className="font-medium text-foreground">Recent recordings</h2>
        <Button variant="ghost" size="sm" onClick={refresh} className="gap-2">
          <RefreshCw className="h-3.5 w-3.5" />
          Refresh
        </Button>
      </div>

      {history.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center gap-3 py-12 text-center">
            <FileAudio className="h-7 w-7 text-muted-foreground" />
            <p className="text-sm text-muted-foreground">
              No recordings yet. Start a session to see it here.
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="flex flex-col gap-2">
          {history.slice(0, 8).map((item) => (
            <RecordingRow
              key={item.session_dir}
              item={item}
              open={expanded === item.session_dir}
              transcribing={rec.transcribingDir === item.session_dir}
              onToggle={() =>
                setExpanded((cur) =>
                  cur === item.session_dir ? null : item.session_dir
                )
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
