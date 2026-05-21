import * as React from "react";
import {
  Mic,
  Square,
  FileAudio,
  FolderOpen,
  RefreshCw,
  ChevronDown,
  ChevronRight,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { AudioPlayer } from "@/components/audio-player";
import { cn, formatBytes, formatDuration } from "@/lib/utils";
import { useRecording } from "@/hooks/use-recording";
import { listRecordings, revealInFinder } from "@/lib/api";
import type { RecordingSummary } from "@/lib/types";

export default function Record() {
  const rec = useRecording();
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

  React.useEffect(() => {
    refresh();
  }, [refresh]);

  React.useEffect(() => {
    if (rec.lastSavedDir) {
      refresh();
      setExpanded(rec.lastSavedDir);
    }
  }, [rec.lastSavedDir, refresh]);

  const elapsedLabel = React.useMemo(() => {
    const m = Math.floor(rec.elapsed / 60);
    const s = rec.elapsed % 60;
    return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
  }, [rec.elapsed]);

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-10">
      <header
        data-drag=""
        className="flex select-none items-baseline justify-between"
      >
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">
            Record
          </h1>
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
              ? `Capturing ${rec.channels.length > 0 ? rec.channels.join(" + ") : "audio"} · transcribe afterward in Library`
              : "Mic + system audio in parallel · transcribe afterward in Library"}
          </p>
          {rec.error && (
            <p className="text-xs text-destructive">{rec.error}</p>
          )}
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
              onToggle={() =>
                setExpanded((cur) =>
                  cur === item.session_dir ? null : item.session_dir
                )
              }
              onReveal={() => revealInFinder(item.session_dir)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function StatusPill({
  recording,
  label,
}: {
  recording: boolean;
  label: string;
}) {
  return (
    <Badge variant="outline" className="gap-2 px-3 py-1 font-mono tracking-tight">
      <span
        className={cn(
          "inline-block h-2 w-2 rounded-full",
          recording
            ? "bg-destructive animate-pulse-record"
            : "border border-muted-foreground"
        )}
      />
      <span>{recording ? "recording" : "idle"}</span>
      <span>·</span>
      <span>{label}</span>
    </Badge>
  );
}

interface RowProps {
  item: RecordingSummary;
  open: boolean;
  onToggle: () => void;
  onReveal: () => void;
}

function RecordingRow({ item, open, onToggle, onReveal }: RowProps) {
  const totalBytes = (item.mic_bytes ?? 0) + (item.system_bytes ?? 0);
  const parts: string[] = [
    formatDuration(item.duration_seconds),
    formatBytes(totalBytes),
  ];

  const micPath = item.mic_bytes ? `${item.session_dir}/mic.wav` : null;
  const systemPath = item.system_bytes
    ? `${item.session_dir}/system.wav`
    : null;

  return (
    <Card className="overflow-hidden">
      <button
        onClick={onToggle}
        className="flex w-full items-center gap-4 px-5 py-3 text-left transition-colors hover:bg-accent/40"
      >
        {open ? (
          <ChevronDown className="h-4 w-4 shrink-0 text-muted-foreground" />
        ) : (
          <ChevronRight className="h-4 w-4 shrink-0 text-muted-foreground" />
        )}
        <FileAudio className="h-4 w-4 shrink-0 text-muted-foreground" />
        <div className="flex min-w-0 flex-1 flex-col">
          <span className="truncate font-mono text-sm">{item.label}</span>
          <span className="font-mono text-2xs text-muted-foreground">
            {parts.join("  ·  ")}
          </span>
        </div>
        <span
          className="ml-auto inline-flex items-center gap-2"
          onClick={(e) => e.stopPropagation()}
        >
          <Button
            variant="ghost"
            size="sm"
            className="gap-2"
            onClick={onReveal}
          >
            <FolderOpen className="h-3.5 w-3.5" />
            Reveal
          </Button>
        </span>
      </button>

      {open && (
        <CardContent className="flex flex-col gap-4 border-t border-border bg-secondary/40 px-5 py-4">
          {micPath ? (
            <AudioPlayer filePath={micPath} label="Mic" />
          ) : (
            <p className="text-xs text-muted-foreground">No mic track.</p>
          )}
          {systemPath && <Separator />}
          {systemPath && <AudioPlayer filePath={systemPath} label="System" />}
        </CardContent>
      )}
    </Card>
  );
}
