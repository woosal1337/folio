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
import { MetaList, MetaRow } from "@/shared/ui/meta-list";
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
            Folio is about to send the recording&apos;s audio to{" "}
            <code>api.openai.com</code>. Confirm before the upload starts.
          </DialogDescription>
        </DialogHeader>

        {payload ? (
          <MetaList>
            <MetaRow
              icon={<HardDrive className="h-4 w-4" />}
              label="Recording"
              value={payload.recordingLabel}
              mono={false}
            />
            <MetaRow
              icon={<Clock className="h-4 w-4" />}
              label="Duration"
              value={formatDuration(payload.estimate.durationMinutes)}
            />
            <MetaRow
              icon={<CloudUpload className="h-4 w-4" />}
              label="Upload size"
              value={formatBytes(payload.estimate.totalBytes)}
            />
            <MetaRow
              icon={<DollarSign className="h-4 w-4" />}
              label="Estimated cost"
              value={formatUsd(payload.estimate.estimatedUsd)}
              hint="charged to your OpenAI key"
            />
          </MetaList>
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
