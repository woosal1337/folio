import * as React from "react";

import { cn } from "@/shared/lib/utils";

interface MetaListProps {
  children: React.ReactNode;
  className?: string;

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
  icon?: React.ReactNode;
  label: React.ReactNode;
  value: React.ReactNode;

  hint?: React.ReactNode;

  mono?: boolean;

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
