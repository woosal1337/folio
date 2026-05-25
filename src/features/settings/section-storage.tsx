import * as React from "react";
import { Archive, Download, Loader2 } from "lucide-react";
import { save as showSaveDialog } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { exportVaultSnapshot } from "@/shared/lib/ipc";
import { formatBytes } from "@/shared/lib/utils";
import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
}

function defaultSnapshotName(): string {
  const now = new Date();
  const pad = (n: number) => n.toString().padStart(2, "0");
  const stamp =
    `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}-` +
    `${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
  return `attune-snapshot-${stamp}.zip`;
}

export function SectionStorage({ settings }: Props) {
  const rows = [
    { label: "Recordings", value: settings.output_dir },
    { label: "Notes", value: settings.notes_dir },
    { label: "Transcripts", value: settings.transcripts_dir },
    { label: "Tasks", value: settings.tasks_path },
  ];

  const [exporting, setExporting] = React.useState(false);

  const handleExport = async () => {
    setExporting(true);
    try {
      const dest = await showSaveDialog({
        defaultPath: defaultSnapshotName(),
        filters: [{ name: "Attune snapshot", extensions: ["zip"] }],
      });
      if (!dest) return; // user cancelled
      const summary = await exportVaultSnapshot(dest);
      toast.success(`Snapshot exported`, {
        description: `${summary.files} files · ${formatBytes(Number(summary.bytes ?? 0))}`,
      });
    } catch (e) {
      console.error("export_vault_snapshot:", e);
      toast.error("Could not export snapshot", { description: String(e) });
    } finally {
      setExporting(false);
    }
  };

  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Storage</h2>
      <p className="text-sm text-muted-foreground">
        All paths are local. Folder pickers land in the next iteration.
      </p>
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
              without our binary, so the export is recoverable without Attune installed.
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
