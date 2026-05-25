/**
 * Modal that asks the user before Attune uploads a recording's WAV to
 * OpenAI Whisper. Rendered once at App root; the recording-store
 * triggers it via `useCloudCostConfirmStore.getState().confirm(...)`.
 *
 * Only fires when the upload exceeds the bandwidth or cost threshold
 * (see cost-estimate.ts). Includes a tip for switching to Local
 * Whisper as the way to avoid future prompts on big meetings.
 *
 * v2 roadmap finding 055.
 */

import * as React from "react";
import { CloudUpload, Cpu, DollarSign, HardDrive, Clock } from "lucide-react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Button } from "@/shared/ui/button";
import { formatBytes, formatDuration, formatUsd } from "@/shared/lib/cost-estimate";
import { useCloudCostConfirmStore } from "@/shared/stores/cloud-cost-confirm-store";

export function CloudCostConfirmDialog() {
  const open = useCloudCostConfirmStore((s) => s.open);
  const payload = useCloudCostConfirmStore((s) => s.payload);
  const resolve = useCloudCostConfirmStore((s) => s.resolve);

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) resolve(false);
      }}
    >
      <DialogContent className="max-w-[480px] p-6">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <CloudUpload className="h-5 w-5 text-primary" />
            Upload this recording to OpenAI Whisper?
          </DialogTitle>
          <DialogDescription>
            Attune is about to send the recording&apos;s audio to{" "}
            <code>api.openai.com</code>. Confirm before the upload starts.
          </DialogDescription>
        </DialogHeader>

        {payload ? (
          <div className="grid gap-3 rounded-lg border border-border bg-secondary/40 p-3 text-sm">
            <Row
              icon={<HardDrive className="h-4 w-4" />}
              label="Recording"
              value={payload.recordingLabel}
            />
            <Row
              icon={<Clock className="h-4 w-4" />}
              label="Duration"
              value={formatDuration(payload.estimate.durationMinutes)}
            />
            <Row
              icon={<CloudUpload className="h-4 w-4" />}
              label="Upload size"
              value={formatBytes(payload.estimate.totalBytes)}
            />
            <Row
              icon={<DollarSign className="h-4 w-4" />}
              label="Estimated cost"
              value={formatUsd(payload.estimate.estimatedUsd)}
              hint="charged to your OpenAI key"
            />
          </div>
        ) : null}

        <p className="rounded-md bg-muted/60 px-3 py-2 text-xs text-muted-foreground">
          <Cpu className="mr-1 inline h-3 w-3" />
          <strong>Tip:</strong> Switch to Local Whisper in Settings → Transcription to
          skip uploads entirely for future recordings of this size.
        </p>

        <DialogFooter className="sm:justify-between">
          <Button variant="ghost" onClick={() => resolve(false)}>
            Cancel
          </Button>
          <Button onClick={() => resolve(true)}>
            <CloudUpload className="mr-2 h-4 w-4" />
            Upload to OpenAI
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Row({
  icon,
  label,
  value,
  hint,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="flex items-center gap-2 text-muted-foreground">
        {icon}
        {label}
      </span>
      <span className="text-right">
        <span className="font-mono text-sm">{value}</span>
        {hint ? (
          <span className="ml-1 text-2xs text-muted-foreground">{hint}</span>
        ) : null}
      </span>
    </div>
  );
}
