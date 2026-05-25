/**
 * Apple/OpenAI-style metadata list: a vertical stack of muted icon +
 * label on the left and a mono value (+ optional tertiary hint) on
 * the right. Used in confirm dialogs, recording metadata cards,
 * agent run cards, settings rows — anywhere we'd otherwise dump a
 * key:value blob.
 *
 * Visual reference: the `CloudCostConfirmDialog` row layout shipped
 * in v2 batch 1, which the user explicitly called out as the
 * template to lift across the app.
 */

import * as React from "react";

import { cn } from "@/shared/lib/utils";

interface MetaListProps {
  children: React.ReactNode;
  className?: string;
  /** When true, wraps the list in a recessed card (the cost-confirm look). */
  cardded?: boolean;
}

export function MetaList({ children, className, cardded = true }: MetaListProps) {
  return (
    <div
      className={cn(
        "grid gap-3 text-sm",
        cardded && "rounded-lg border border-border bg-secondary/40 p-3",
        className
      )}
    >
      {children}
    </div>
  );
}

interface MetaRowProps {
  /** Lucide icon or any 14-16px glyph. */
  icon?: React.ReactNode;
  label: React.ReactNode;
  value: React.ReactNode;
  /** Tertiary muted text trailing the value (e.g. "charged to your OpenAI key"). */
  hint?: React.ReactNode;
  /** When true, render the value in tabular-numeric monospace (the default for sizes / counts / costs). */
  mono?: boolean;
  /** Optional extra class on the row. */
  className?: string;
}

export function MetaRow({
  icon,
  label,
  value,
  hint,
  mono = true,
  className,
}: MetaRowProps) {
  return (
    <div className={cn("flex items-center justify-between gap-3", className)}>
      <span className="flex min-w-0 items-center gap-2 text-muted-foreground">
        {icon ? <span className="inline-flex h-4 w-4 shrink-0">{icon}</span> : null}
        <span className="truncate">{label}</span>
      </span>
      <span className="text-right">
        <span className={cn("text-sm", mono && "font-mono tabular-nums")}>{value}</span>
        {hint ? (
          <span className="ml-1 text-2xs text-muted-foreground">{hint}</span>
        ) : null}
      </span>
    </div>
  );
}
