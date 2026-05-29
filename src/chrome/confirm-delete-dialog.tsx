/**
 * Confirmation modal for destructive actions. Mounted once at App root;
 * any code path triggers it via `confirmDelete({ ... })` and awaits the
 * result. The dialog itself is the confirmation — nothing is removed
 * until the user clicks the destructive button.
 */

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

        <DialogFooter className="sm:justify-between">
          <Button variant="ghost" onClick={() => resolve(false)}>
            Cancel
          </Button>
          <Button variant="destructive" onClick={() => resolve(true)}>
            {payload?.confirmLabel ?? "Delete"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
