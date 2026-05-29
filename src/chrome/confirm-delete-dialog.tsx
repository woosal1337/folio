/**
 * Confirmation modal for destructive actions. Mounted once at App root;
 * any code path triggers it via `confirmDelete({ ... })` and awaits the
 * result. For irreversible deletes (`doubleConfirm`), the destructive
 * button stays disabled until the user ticks "I understand" — the second
 * confirmation, so nothing is removed on a single stray click.
 */

import * as React from "react";
import { AlertTriangle } from "lucide-react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Button } from "@/shared/ui/button";
import { useConfirmDeleteStore } from "@/shared/stores/confirm-delete-store";

export function ConfirmDeleteDialog() {
  const open = useConfirmDeleteStore((s) => s.open);
  const payload = useConfirmDeleteStore((s) => s.payload);
  const resolve = useConfirmDeleteStore((s) => s.resolve);

  const [acknowledged, setAcknowledged] = React.useState(false);

  // Reset the acknowledgement each time a new confirmation opens.
  React.useEffect(() => {
    if (open) setAcknowledged(false);
  }, [open, payload]);

  const doubleConfirm = payload?.doubleConfirm ?? false;
  const confirmDisabled = doubleConfirm && !acknowledged;

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) resolve(false);
      }}
    >
      <DialogContent className="max-w-[440px] p-6">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-destructive" />
            {payload?.title ?? "Delete?"}
          </DialogTitle>
          <DialogDescription className="whitespace-pre-line">
            {payload?.description}
          </DialogDescription>
        </DialogHeader>

        {doubleConfirm ? (
          <label className="flex items-start gap-2 rounded-md border border-border bg-muted/40 px-3 py-2 text-sm">
            <input
              type="checkbox"
              checked={acknowledged}
              onChange={(e) => setAcknowledged(e.target.checked)}
              className="mt-0.5 h-4 w-4 accent-[hsl(var(--destructive))]"
            />
            <span className="text-muted-foreground">
              {payload?.acknowledgeLabel ?? "I understand this can't be undone."}
            </span>
          </label>
        ) : null}

        <DialogFooter className="sm:justify-between">
          <Button variant="ghost" onClick={() => resolve(false)}>
            Cancel
          </Button>
          <Button
            variant="destructive"
            disabled={confirmDisabled}
            onClick={() => resolve(true)}
          >
            {payload?.confirmLabel ?? "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
