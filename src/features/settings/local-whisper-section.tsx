import * as React from "react";
import { onWhisperDownloadProgress } from "@/shared/lib/ipc";
import { Cpu, Download, Loader2, RefreshCw } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import { humanizeError } from "@/shared/lib/errors";
import { cn, formatBytes } from "@/shared/lib/utils";
import { ensureWhisperModel, type WhisperDownloadProgress } from "@/shared/lib/ipc";
import type { Settings } from "@/shared/types/Settings";
import type { WhisperModel } from "@/shared/types/WhisperModel";
import type { WhisperModelStatus } from "@/shared/types/WhisperModelStatus";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
  status: WhisperModelStatus | null;
  statusLoading: boolean;
  refreshStatus: () => void;
  onStatusChange: (status: WhisperModelStatus) => void;
}

const MODELS: { id: WhisperModel; label: string; size: string; note: string }[] = [
  { id: "tiny", label: "Tiny", size: "~75 MB", note: "fastest, lowest quality" },
  { id: "base", label: "Base", size: "~142 MB", note: "fast, basic" },
  { id: "small", label: "Small", size: "~466 MB", note: "good balance" },
  { id: "medium", label: "Medium", size: "~1.5 GB", note: "better, slower" },
  { id: "large-v3", label: "Large v3", size: "~3.1 GB", note: "best quality" },
];

export function LocalWhisperSection({
  settings,
  onChange,
  status,
  statusLoading,
  refreshStatus,
  onStatusChange,
}: Props) {
  const [downloading, setDownloading] = React.useState(false);
  const [progress, setProgress] = React.useState<WhisperDownloadProgress | null>(null);

  React.useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    let cancelled = false;
    (async () => {
      const unlisten = await onWhisperDownloadProgress<WhisperDownloadProgress>(
        (payload) => {
          if (cancelled) return;
          setProgress(payload);
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
      onStatusChange(next);
      toast.success("Whisper model ready", {
        description: `${formatBytes(Number(next.bytes_on_disk ?? 0n))} on disk.`,
      });
    } catch (e) {
      console.error("ensure_whisper_model:", e);
      toast.error("Could not download model", { description: humanizeError(e) });
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

      <div className="flex flex-col gap-3 rounded-md border border-border bg-card p-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex min-w-0 items-center gap-2">
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

          <div className="flex shrink-0 items-center gap-1">
            {!downloading && (
              <Button
                variant="ghost"
                size="sm"
                onClick={refreshStatus}
                aria-label="Refresh model status"
                className="h-8 px-2"
                disabled={statusLoading}
              >
                <RefreshCw className="h-3.5 w-3.5" />
              </Button>
            )}
            <Button
              variant="outline"
              size="sm"
              onClick={handleDownload}
              disabled={downloading || statusLoading}
              className="h-8 gap-1.5 px-3 text-xs"
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

        <div className="min-w-0">
          {downloading && progress ? (
            <div className="flex flex-col gap-1" role="status" aria-live="polite">
              <span className="font-mono text-2xs text-muted-foreground">
                {formatBytes(progress.downloaded)}
                {progress.total ? ` / ${formatBytes(progress.total)}` : ""}
                {percent !== null ? ` · ${percent}%` : ""}
              </span>
              <div className="h-1.5 w-full overflow-hidden rounded-full bg-secondary">
                <div
                  className="h-full w-full origin-left bg-primary transition-transform"
                  style={{ transform: `scaleX(${(percent ?? 0) / 100})` }}
                />
              </div>
            </div>
          ) : status?.present ? (
            <p
              className="break-all font-mono text-2xs leading-relaxed text-muted-foreground"
              title={status.path}
            >
              <span className="text-muted-foreground/80">
                {formatBytes(Number(status.bytes_on_disk ?? 0n))} ·
              </span>{" "}
              {status.path}
            </p>
          ) : (
            <p
              className="break-all font-mono text-2xs leading-relaxed text-muted-foreground"
              title={status?.path}
            >
              Will download to {status?.path ?? "your application support folder"}
            </p>
          )}
        </div>
      </div>

      <p className="text-xs text-muted-foreground">
        Models are fetched from huggingface.co/ggerganov/whisper.cpp and cached locally.
        Large v3 (~3.1 GB) gives the best quality and runs ~realtime on Apple Silicon.
      </p>
    </section>
  );
}
