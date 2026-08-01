import * as React from "react";
import {
  Archive,
  CalendarRange,
  Download,
  GitBranch,
  Loader2,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { showSaveDialog } from "@/shared/lib/ipc";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import {
  exportVaultSnapshot,
  generateWeeklyDigest,
  gitSyncVault,
  gitVaultIsRepo,
  purgeOldWavFiles,
} from "@/shared/lib/ipc";
import { humanizeError } from "@/shared/lib/errors";
import { formatBytes } from "@/shared/lib/utils";
import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

function defaultSnapshotName(): string {
  const now = new Date();
  const pad = (n: number) => n.toString().padStart(2, "0");
  const stamp =
    `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}-` +
    `${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `folio-snapshot-${stamp}.zip`;
}

export function SectionStorage({ settings, onChange }: Props) {
  const rows = [
    { label: "Recordings", value: settings.output_dir },
    { label: "Tasks", value: settings.tasks_path },
  ];

  const [exporting, setExporting] = React.useState(false);
  const [purging, setPurging] = React.useState(false);
  const [digesting, setDigesting] = React.useState(false);
  const [isRepo, setIsRepo] = React.useState<boolean | null>(null);
  const [syncing, setSyncing] = React.useState(false);
  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const value = await gitVaultIsRepo();
        if (!cancelled) setIsRepo(value);
      } catch (e) {
        if (!cancelled) {
          console.error("git_vault_is_repo:", e);
          setIsRepo(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);
  const handleSync = async () => {
    setSyncing(true);
    try {
      const summary = await gitSyncVault();
      if (!summary.is_repo) {
        toast.info("Vault is not a git repository", {
          description: "Run `git init` in the vault dir to enable sync.",
        });
        return;
      }
      if (summary.ok) {
        toast.success(
          `Vault synced on ${summary.branch}${summary.committed ? " (commit pushed)" : " (no local changes)"}`
        );
      } else {
        toast.error("Git sync failed", {
          description:
            (summary.pull_log || summary.push_log).slice(0, 200) || "see logs",
        });
      }
    } catch (e) {
      console.error("git_sync_vault:", e);
      toast.error("Could not sync vault", { description: humanizeError(e) });
    } finally {
      setSyncing(false);
    }
  };
  const handleDigest = async () => {
    setDigesting(true);
    try {
      const result = await generateWeeklyDigest();
      toast.success("Digest generated", {
        description: `${result.recordings} recordings · ${result.aged_tasks} aged tasks · ${result.new_memories} new memories`,
      });
    } catch (e) {
      console.error("generate_weekly_digest:", e);
      toast.error("Could not generate digest", { description: humanizeError(e) });
    } finally {
      setDigesting(false);
    }
  };
  const retentionDays = settings.wav_retention_days ?? null;
  const handleRetentionChange = (raw: string) => {
    const n = parseInt(raw, 10);
    if (raw.trim() === "" || Number.isNaN(n) || n <= 0) {
      onChange("wav_retention_days", null);
    } else {
      onChange("wav_retention_days", Math.min(n, 3650));
    }
  };
  const handlePurge = async () => {
    const days = retentionDays;
    if (!days || days <= 0) {
      toast.info("Set a retention threshold first", {
        description: "Enter the number of days WAVs may live after transcription.",
      });
      return;
    }
    if (
      !window.confirm(
        `Delete mic.wav + system.wav from every transcribed recording older than ${days} day${days === 1 ? "" : "s"}? Transcripts and agent runs stay.`
      )
    ) {
      return;
    }
    setPurging(true);
    try {
      const summary = await purgeOldWavFiles(days);
      toast.success("WAV purge complete", {
        description: `${summary.wavs_deleted} files · ${formatBytes(Number(summary.bytes_freed ?? 0))} freed`,
      });
    } catch (e) {
      console.error("purge_old_wav_files:", e);
      toast.error("Could not purge WAVs", { description: humanizeError(e) });
    } finally {
      setPurging(false);
    }
  };

  const handleExport = async () => {
    setExporting(true);
    try {
      const dest = await showSaveDialog({
        defaultPath: defaultSnapshotName(),
        filters: [{ name: "Folio snapshot", extensions: ["zip"] }],
      });
      if (!dest) return;
      const summary = await exportVaultSnapshot(dest);
      toast.success(`Snapshot exported`, {
        description: `${summary.files} files · ${formatBytes(Number(summary.bytes ?? 0))}`,
      });
    } catch (e) {
      console.error("export_vault_snapshot:", e);
      toast.error("Could not export snapshot", { description: humanizeError(e) });
    } finally {
      setExporting(false);
    }
  };

  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Storage</h2>
      <p className="text-sm text-muted-foreground">All paths are local.</p>
      <div className="grid gap-3">
        {rows.map((r) => (
          <div
            key={r.label}
            className="flex items-center justify-between gap-4 rounded-lg border border-border bg-card p-4"
          >
            <div>
              <p className="text-sm font-medium">{r.label}</p>
              <p className="mt-0.5 break-all font-mono text-xs text-muted-foreground">
                {r.value}
              </p>
            </div>
          </div>
        ))}
      </div>

      <section
        aria-label="WAV retention"
        className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4"
      >
        <div className="flex items-start gap-3">
          <Trash2 className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="flex-1">
            <p className="text-sm font-medium">WAV retention</p>
            <p className="mt-0.5 text-xs text-muted-foreground">
              Delete the source mic + system WAV files once a transcript exists and the
              audio is at least this many days old. Transcripts and agent runs stay
              forever. Leave blank to keep every WAV until you delete the recording
              yourself. A daily-meeting user accumulates ~87 GB of WAVs per year — this
              is the biggest disk-saving knob the app exposes.
            </p>
            <div className="mt-3 flex items-center gap-2">
              <Input
                inputMode="numeric"
                pattern="[0-9]*"
                value={retentionDays === null ? "" : String(retentionDays)}
                onChange={(e) => handleRetentionChange(e.target.value)}
                placeholder="14"
                className="h-8 w-24 tabular-nums"
                aria-label="Retention days"
              />
              <span className="text-xs text-muted-foreground">days</span>
              <Button
                variant="outline"
                size="sm"
                disabled={purging || !retentionDays}
                onClick={handlePurge}
                className="ml-auto gap-2"
              >
                {purging ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Trash2 className="h-3.5 w-3.5" />
                )}
                {purging ? "Purging…" : "Purge now"}
              </Button>
            </div>
          </div>
        </div>
      </section>

      <section
        aria-label="Git vault sync"
        className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4"
      >
        <div className="flex items-start gap-3">
          <GitBranch className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="flex-1">
            <p className="text-sm font-medium">Multi-machine sync via git</p>
            <p className="mt-0.5 text-xs text-muted-foreground">
              {isRepo
                ? "Your memory dir is a git repo. Sync runs git pull --rebase, commits any local changes, and pushes — no Folio cloud."
                : "Your memory dir is not (yet) a git repo. Run `git init` + add a remote inside it to enable sync."}
            </p>
          </div>
        </div>
        <div className="flex justify-end">
          <Button
            variant="outline"
            size="sm"
            disabled={syncing || !isRepo}
            onClick={handleSync}
            className="gap-2"
          >
            {syncing ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <RefreshCw className="h-3.5 w-3.5" />
            )}
            {syncing ? "Syncing…" : "Sync now"}
          </Button>
        </div>
      </section>

      <section
        aria-label="Weekly digest"
        className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4"
      >
        <div className="flex items-start gap-3">
          <CalendarRange className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="flex-1">
            <p className="text-sm font-medium">Weekly digest</p>
            <p className="mt-0.5 text-xs text-muted-foreground">
              Writes a markdown summary of the last 7 days to{" "}
              <code className="rounded bg-muted px-1 py-0.5 text-2xs">
                ~/Documents/Folio/Digests/
              </code>
              : meetings, tasks aging more than a week, and new memories. Designed to
              drop into Obsidian or skim from your dock. Background scheduling (Sunday
              6pm) is a follow-up.
            </p>
          </div>
        </div>
        <div className="flex justify-end">
          <Button
            variant="outline"
            size="sm"
            disabled={digesting}
            onClick={handleDigest}
            className="gap-2"
          >
            {digesting ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <CalendarRange className="h-3.5 w-3.5" />
            )}
            {digesting ? "Generating…" : "Generate digest"}
          </Button>
        </div>
      </section>

      <section
        aria-label="Vault snapshot"
        className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4"
      >
        <div className="flex items-start gap-3">
          <Archive className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="flex-1">
            <p className="text-sm font-medium">Vault snapshot</p>
            <p className="mt-0.5 text-xs text-muted-foreground">
              Bundles your settings, tasks, recordings, and memory into a single zip you
              can drop into iCloud, Dropbox, or a USB stick. Plain{" "}
              <code className="rounded bg-muted px-1 py-0.5 text-2xs">unzip</code> works
              without our binary, so the export is recoverable without Folio installed.
            </p>
          </div>
        </div>
        <div className="flex justify-end">
          <Button
            variant="outline"
            size="sm"
            disabled={exporting}
            onClick={handleExport}
            className="gap-2"
          >
            {exporting ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <Download className="h-3.5 w-3.5" />
            )}
            {exporting ? "Exporting…" : "Export snapshot"}
          </Button>
        </div>
      </section>
    </div>
  );
}
