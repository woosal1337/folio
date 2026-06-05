import * as React from "react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/shared/ui/dialog";
import { SHORTCUTS, formatChord, type Shortcut } from "@/shared/lib/shortcuts";

interface Props {
  open: boolean;
  onClose: () => void;
}

const GROUP_ORDER: Shortcut["group"][] = ["Recording", "Navigation", "Editing", "Help"];

export function CheatsheetOverlay({ open, onClose }: Props) {
  const grouped = React.useMemo(() => {
    const out: Record<string, Shortcut[]> = {};
    for (const s of SHORTCUTS) {
      const bucket = out[s.group] ?? [];
      bucket.push(s);
      out[s.group] = bucket;
    }
    return out;
  }, []);

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-2xl p-6">
        <DialogTitle className="font-serif text-xl font-medium">
          Keyboard shortcuts
        </DialogTitle>
        <DialogDescription className="text-sm text-muted-foreground">
          Press Esc to close.
        </DialogDescription>
        <div className="mt-4 grid grid-cols-2 gap-x-8 gap-y-6">
          {GROUP_ORDER.map((group) => {
            const items = grouped[group];
            if (!items?.length) return null;
            return (
              <section key={group}>
                <h2 className="mb-2 text-2xs uppercase tracking-wider text-muted-foreground">
                  {group}
                </h2>
                <dl className="flex flex-col gap-1.5">
                  {items.map((s) => (
                    <div
                      key={s.action}
                      className="flex items-baseline justify-between gap-3"
                    >
                      <dt className="text-sm text-foreground">{s.label}</dt>
                      <dd className="rounded border border-border bg-muted px-1.5 py-0.5 font-mono text-xs text-foreground">
                        {formatChord(s.keys)}
                      </dd>
                    </div>
                  ))}
                </dl>
              </section>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}
