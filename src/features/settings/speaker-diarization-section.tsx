import * as React from "react";
import { Check, Download, Loader2, RefreshCw, Users } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import { humanizeError } from "@/shared/lib/errors";
import { formatBytes } from "@/shared/lib/utils";
import {
  diarizationModelStatus,
  type DiarizationDownloadProgress,
  ensureDiarizationModels,
  onDiarizationDownloadProgress,
} from "@/shared/lib/ipc";
import type { DiarizationModelStatus } from "@/shared/types/DiarizationModelStatus";
import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

export function SpeakerDiarizationSection({ settings, onChange }: Props) {
  const [statuses, setStatuses] = React.useState<DiarizationModelStatus[] | null>(null);
  const [statusLoading, setStatusLoading] = React.useState(true);
  const [downloading, setDownloading] = React.useState(false);
  const [progress, setProgress] = React.useState<DiarizationDownloadProgress | null>(
    null
  );

  const refreshStatus = React.useCallback(async () => {
    setStatusLoading(true);
    try {
      const s = await diarizationModelStatus();
      setStatuses(s);
    } catch (e) {
      console.error("diarization_model_status:", e);
      toast.error("Could not read speaker-model status", {
        description: humanizeError(e),
      });
    } finally {
      setStatusLoading(false);
    }
  }, []);

  React.useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  React.useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    let cancelled = false;
    (async () => {
      const unlisten = await onDiarizationDownloadProgress<DiarizationDownloadProgress>(
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

  const allPresent =
    statuses !== null && statuses.length > 0 && statuses.every((s) => s.present);

  const handleDownload = async () => {
    setDownloading(true);
    setProgress(null);
    try {
      const next = await ensureDiarizationModels();
      setStatuses(next);
      toast.success("Speaker models ready", {
        description: "Diarization will run on your next transcription.",
      });
    } catch (e) {
      console.error("ensure_diarization_models:", e);
      toast.error("Could not download speaker models", {
        description: humanizeError(e),
      });
    } finally {
      setDownloading(false);
      setProgress(null);
    }
  };

  const active = progress
    ? (statuses?.find((s) => s.id === progress.model_id) ?? null)
    : null;
  const activeLabel = active?.label ?? progress?.model_id ?? null;
  const totalBytes =
    progress?.total ?? (active ? Number(active.approx_total_bytes) : 0);
  const percent =
    progress && totalBytes > 0
      ? Math.min(100, Math.floor((progress.downloaded / totalBytes) * 100))
      : null;

  return (
    <section className="space-y-3">
      <div className="flex items-start justify-between gap-6 rounded-lg border border-border bg-card p-4">
        <div className="space-y-1">
          <Label
            htmlFor="diarization-toggle"
            className="flex items-center gap-2 text-sm font-medium"
          >
            <Users className="h-4 w-4 text-muted-foreground" />
            Detect speakers
            <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
              Recommended
            </span>
          </Label>
          <p className="max-w-md text-xs text-muted-foreground">
            After transcribing, Folio tells apart who spoke on the system-audio track
            and labels each turn Speaker 1, 2, 3… Your own microphone is always labelled
            “You”. Runs fully on-device and needs the two models below.
          </p>
          {!statusLoading && !allPresent ? (
            <p className="text-2xs font-medium text-amber-600 dark:text-amber-400">
              Download the speaker models below to turn this on.
            </p>
          ) : null}
        </div>
        <Switch
          id="diarization-toggle"
          checked={settings.diarization_enabled && allPresent}
          disabled={statusLoading || !allPresent}
          onCheckedChange={(checked) => onChange("diarization_enabled", checked)}
          className="mt-1"
        />
      </div>

      <div className="flex flex-col gap-3 rounded-md border border-border bg-card p-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex min-w-0 items-center gap-2">
            <span className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
              Speaker models
            </span>
            {statusLoading ? (
              <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
            ) : allPresent ? (
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
                aria-label="Refresh speaker-model status"
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
              disabled={downloading || statusLoading || allPresent}
              className="h-8 gap-1.5 px-3 text-xs"
            >
              {downloading ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : allPresent ? (
                <Check className="h-3.5 w-3.5" />
              ) : (
                <Download className="h-3.5 w-3.5" />
              )}
              {downloading ? "Downloading…" : allPresent ? "Installed" : "Download"}
            </Button>
          </div>
        </div>

        <ul className="flex flex-col gap-1.5">
          {(statuses ?? []).map((m) => (
            <li key={m.id} className="flex items-center justify-between gap-3 text-xs">
              <span className="flex min-w-0 items-center gap-2">
                {m.present ? (
                  <Check className="h-3.5 w-3.5 shrink-0 text-emerald-500" />
                ) : (
                  <Download className="h-3.5 w-3.5 shrink-0 text-muted-foreground/60" />
                )}
                <span className="truncate font-medium">{m.label}</span>
              </span>
              <span className="shrink-0 font-mono text-2xs text-muted-foreground">
                {m.present
                  ? formatBytes(Number(m.bytes_on_disk ?? 0n))
                  : `~${formatBytes(Number(m.approx_total_bytes))}`}
              </span>
            </li>
          ))}
        </ul>

        {downloading && progress ? (
          <div className="flex flex-col gap-1" role="status" aria-live="polite">
            <span className="font-mono text-2xs text-muted-foreground">
              {activeLabel ? `${activeLabel} · ` : ""}
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
        ) : null}
      </div>

      <p className="text-xs text-muted-foreground">
        Fetched once from huggingface.co (pyannote-segmentation-3.0) and
        github.com/k2-fsa/sherpa-onnx (WeSpeaker), ~32 MB total, cached locally. Each
        file is checksum-verified on download.
      </p>
    </section>
  );
}
