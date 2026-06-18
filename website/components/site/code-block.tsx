"use client";

import * as React from "react";
import { Check, Copy } from "lucide-react";

import { cn } from "@/lib/utils";

export function CopyButton({ value, className }: { value: string; className?: string }) {
  const [copied, setCopied] = React.useState(false);

  const copy = React.useCallback(() => {
    void navigator.clipboard.writeText(value).then(() => {
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1600);
    });
  }, [value]);

  return (
    <button
      type="button"
      onClick={copy}
      aria-label={copied ? "Copied" : "Copy to clipboard"}
      className={cn(
        "inline-flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground focus-ring",
        className
      )}
    >
      {copied ? (
        <Check className="h-3.5 w-3.5 text-primary" />
      ) : (
        <Copy className="h-3.5 w-3.5" />
      )}
    </button>
  );
}

export function CodeBlock({
  code,
  label,
  className,
}: {
  code: string;
  label?: string;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "group relative overflow-hidden rounded-lg border border-border bg-card shadow-sm",
        className
      )}
    >
      <div className="flex items-center justify-between border-b border-border bg-secondary/60 px-4 py-2">
        <span className="font-mono text-2xs uppercase tracking-[0.16em] text-muted-foreground">
          {label ?? "shell"}
        </span>
        <CopyButton value={code} />
      </div>
      <pre className="overflow-x-auto px-4 py-3.5 text-ms-13 leading-relaxed">
        <code className="font-mono text-foreground">{code}</code>
      </pre>
    </div>
  );
}

export function CommandLine({ command, className }: { command: string; className?: string }) {
  return (
    <div
      className={cn(
        "flex items-center justify-between gap-3 rounded-lg border border-border bg-card px-4 py-2.5 shadow-sm",
        className
      )}
    >
      <code className="flex items-center gap-2 overflow-x-auto font-mono text-ms-13 text-foreground">
        <span className="select-none text-primary" aria-hidden>
          $
        </span>
        {command}
      </code>
      <CopyButton value={command} />
    </div>
  );
}
