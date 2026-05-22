import * as React from "react";
import { listen } from "@tauri-apps/api/event";
import { Cpu, Download, Loader2, RefreshCw } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import { cn, formatBytes } from "@/shared/lib/utils";
import {
  ensureWhisperModel,
  WHISPER_DOWNLOAD_PROGRESS_EVENT,
  whisperModelStatus,
  type WhisperDownloadProgress,
} from "@/shared/lib/ipc";
import type { Settings } from "@/shared/types/Settings";
import type { WhisperModel } from "@/shared/types/WhisperModel";
import type { WhisperModelStatus } from "@/shared/types/WhisperModelStatus";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

const MODELS: { id: WhisperModel; label: string; size: string; note: string }[] = [
  { id: "tiny", label: "Tiny", size: "~75 MB", note: "fastest, lowest quality" },
  { id: "base", label: "Base", size: "~142 MB", note: "fast, basic" },
  { id: "small", label: "Small", size: "~466 MB", note: "good balance" },
  { id: "medium", label: "Medium", size: "~1.5 GB", note: "better, slower" },
  { id: "large-v3", label: "Large v3", size: "~3.1 GB", note: "best quality" },
];

/**
 * The local-Whisper subsection of the Transcription settings panel.
 * Only rendered when `settings.transcriber === "local_whisper"`.
 *
 * Owns the model picker, the current model's download status, and the
 * live download progress feed (subscribed via Tauri's event channel
 * while the section is mounted).
 */
export function LocalWhisperSection({ settings, onChange }: Props) {
  const [status, setStatus] = React.useState<WhisperModelStatus | null>(null);
  const [statusLoading, setStatusLoading] = React.useState(true);
  const [downloading, setDownloading] = React.useState(false);
  const [progress, setProgress] = React.useState<WhisperDownloadProgress | null>(null);

  const refreshStatus = React.useCallback(async () => {
    setStatusLoading(true);
    try {
      const s = await whisperModelStatus();
      setStatus(s);
    } catch (e) {
      console.error("whisper_model_status:", e);
      toast.error("Could not read model status", { description: String(e) });
    } finally {
      setStatusLoading(false);
    }
  }, []);

  // Re-read status whenever the selected model changes.
  React.useEffect(() => {
    refreshStatus();
  }, [refreshStatus, settings.local_whisper_model]);

  // Subscribe to live download progress while this section is mounted.
  React.useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    let cancelled = false;
    (async () => {
      const unlisten = await listen<WhisperDownloadProgress>(
        WHISPER_DOWNLOAD_PROGRESS_EVENT,
        (event) => {
          if (cancelled) return;
          setProgress(event.payload);
        }
      );
      if (cancelled) {
        unlisten();
      } else {
        unlistenFn = unlisten;
      }
    })();
    return () => {
      cancelled = true;
      if (unlistenFn) unlistenFn();
    };
  }, []);

  const handleDownload = async () => {
    setDownloading(true);
    setProgress(null);
    try {
      const next = await ensureWhisperModel(
        settings.local_whisper_model as WhisperModel
      );
      setStatus(next);
      toast.success("Whisper model ready", {
        description: `${formatBytes(Number(next.bytes_on_disk ?? 0n))} on disk.`,
      });
    } catch (e) {
      console.error("ensure_whisper_model:", e);
      toast.error("Could not download model", { description: String(e) });
    } finally {
      setDownloading(false);
      setProgress(null);
    }
  };

  const totalBytes = progress?.total ?? Number(status?.approx_total_bytes ?? 0n);
  const percent =
    progress && totalBytes > 0
      ? Math.min(100, Math.floor((progress.downloaded / totalBytes) * 100))
      : null;

  return (
    <section className="space-y-3">
      <Label className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
        <Cpu className="h-3.5 w-3.5" />
        Local model
      </Label>

      <div className="grid gap-1.5">
        {MODELS.map((m) => {
          const selected = settings.local_whisper_model === m.id;
          return (
            <button
              type="button"
              key={m.id}
              onClick={() => onChange("local_whisper_model", m.id)}
              aria-pressed={selected}
              className={cn(
                "flex items-center justify-between gap-3 rounded-md border px-3 py-2 text-left transition-colors",
                selected
                  ? "border-primary bg-accent"
                  : "border-border bg-card hover:bg-secondary"
              )}
            >
              <div className="flex min-w-0 items-baseline gap-2.5">
                <span className="text-sm font-medium">{m.label}</span>
                <span className="truncate text-xs text-muted-foreground">{m.note}</span>
              </div>
              <span className="shrink-0 font-mono text-2xs text-muted-foreground">
                {m.size}
              </span>
            </button>
          );
        })}
      </div>

      <div className="flex items-center justify-between gap-3 rounded-md border border-border bg-card p-3">
        <div className="flex min-w-0 flex-col gap-1">
          <div className="flex items-center gap-2">
            <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
              Status
            </span>
            {statusLoading ? (
              <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
            ) : status?.present ? (
              <Badge variant="accent" className="text-2xs">
                ready
              </Badge>
            ) : (
              <Badge variant="outline" className="text-2xs">
                not downloaded
              </Badge>
            )}
          </div>
          {downloading && progress ? (
            <div className="flex flex-col gap-1" role="status" aria-live="polite">
              <span className="font-mono text-2xs text-muted-foreground">
                {formatBytes(progress.downloaded)}
                {progress.total ? ` / ${formatBytes(progress.total)}` : ""}
                {percent !== null ? ` · ${percent}%` : ""}
              </span>
              <div className="h-1.5 w-full overflow-hidden rounded-full bg-secondary">
                <div
                  className="h-full bg-primary transition-all"
                  style={{ width: `${percent ?? 0}%` }}
                />
              </div>
            </div>
          ) : status?.present ? (
            <p className="truncate font-mono text-2xs text-muted-foreground">
              {formatBytes(Number(status.bytes_on_disk ?? 0n))} · {status.path}
            </p>
          ) : (
            <p className="font-mono text-2xs text-muted-foreground">
              Will download to {status?.path ?? "your application support folder"}
            </p>
          )}
        </div>

        <div className="flex items-center gap-1">
          {!downloading && (
            <Button
              variant="ghost"
              size="sm"
              onClick={refreshStatus}
              aria-label="Refresh model status"
              className="gap-2"
              disabled={statusLoading}
            >
              <RefreshCw className="h-3.5 w-3.5" />
            </Button>
          )}
          <Button
            onClick={handleDownload}
            disabled={downloading || statusLoading}
            className="gap-2"
          >
            {downloading ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <Download className="h-3.5 w-3.5" />
            )}
            {downloading
              ? "Downloading…"
              : status?.present
                ? "Re-download"
                : "Download"}
          </Button>
        </div>
      </div>

      <p className="text-xs text-muted-foreground">
        Models are fetched from huggingface.co/ggerganov/whisper.cpp and cached locally.
        Large v3 (~3.1 GB) gives the best quality and runs ~realtime on Apple Silicon.
      </p>
    </section>
  );
}
