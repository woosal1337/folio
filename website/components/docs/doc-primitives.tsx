import * as React from "react";
import Link from "next/link";
import { AlertTriangle, ArrowUpRight, Info, Lightbulb, ShieldAlert } from "lucide-react";

import { cn } from "@/lib/utils";
import { Eyebrow } from "@/components/site/section";

export function DocHeader({
  eyebrow,
  title,
  description,
}: {
  eyebrow?: string;
  title: string;
  description?: React.ReactNode;
}) {
  return (
    <header className="flex flex-col gap-4 border-b border-border pb-8">
      {eyebrow && <Eyebrow>{eyebrow}</Eyebrow>}
      <h1 className="text-balance font-display text-ms-34 font-semibold tracking-tight sm:text-ms-45">
        {title}
      </h1>
      {description && (
        <p className="text-pretty text-ms-17 leading-relaxed text-muted-foreground">
          {description}
        </p>
      )}
    </header>
  );
}

export function DocH2({ id, children }: { id: string; children: React.ReactNode }) {
  return (
    <h2
      id={id}
      className="group scroll-mt-24 pt-4 font-display text-ms-22 font-semibold tracking-tight"
    >
      <a href={`#${id}`} className="no-underline">
        {children}
        <span className="ml-2 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100">
          #
        </span>
      </a>
    </h2>
  );
}

export function DocH3({ id, children }: { id?: string; children: React.ReactNode }) {
  return (
    <h3 id={id} className="scroll-mt-24 text-ms-17 font-semibold tracking-tight">
      {children}
    </h3>
  );
}

export function Prose({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      className={cn(
        "text-ms-15 leading-relaxed text-muted-foreground",
        "[&_p]:mb-4 [&_p]:last:mb-0",
        "[&_strong]:font-semibold [&_strong]:text-foreground",
        "[&_a]:font-medium [&_a]:text-foreground [&_a]:underline [&_a]:decoration-muted-foreground/40 [&_a]:underline-offset-2 hover:[&_a]:decoration-foreground",
        "[&_ul]:mb-4 [&_ul]:ml-5 [&_ul]:list-disc [&_ul]:space-y-2 [&_ul]:marker:text-muted-foreground/60",
        "[&_ol]:mb-4 [&_ol]:ml-5 [&_ol]:list-decimal [&_ol]:space-y-2 [&_ol]:marker:text-muted-foreground/60",
        "[&_li]:pl-1.5 [&_li>strong]:text-foreground",
        "[&_code]:rounded [&_code]:bg-muted [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[0.85em] [&_code]:text-foreground",
        "[&_blockquote]:my-4 [&_blockquote]:border-l-2 [&_blockquote]:border-primary/40 [&_blockquote]:pl-4 [&_blockquote]:italic",
        className
      )}
    >
      {children}
    </div>
  );
}

const calloutStyles = {
  note: {
    icon: Info,
    container: "border-border bg-secondary/60",
    accent: "text-muted-foreground",
  },
  tip: {
    icon: Lightbulb,
    container: "border-border bg-card",
    accent: "text-foreground",
  },
  warning: {
    icon: AlertTriangle,
    container: "border-foreground/15 bg-muted/70",
    accent: "text-foreground",
  },
  privacy: {
    icon: ShieldAlert,
    container: "border-foreground/25 bg-secondary",
    accent: "text-foreground",
  },
} as const;

export function Callout({
  variant = "note",
  title,
  children,
}: {
  variant?: keyof typeof calloutStyles;
  title?: string;
  children: React.ReactNode;
}) {
  const style = calloutStyles[variant];
  const Icon = style.icon;
  return (
    <div className={cn("my-6 flex gap-3 rounded-xl border p-4", style.container)}>
      <Icon className={cn("mt-0.5 h-4 w-4 shrink-0", style.accent)} />
      <div className="flex flex-col gap-1 text-ms-15 leading-relaxed text-muted-foreground">
        {title && <p className="font-semibold text-foreground">{title}</p>}
        <div className="[&_a]:font-medium [&_a]:text-primary [&_a]:underline-offset-2 hover:[&_a]:underline">
          {children}
        </div>
      </div>
    </div>
  );
}

export function Steps({ children }: { children: React.ReactNode }) {
  return (
    <ol className="my-6 flex flex-col gap-6 border-l border-border pl-8">{children}</ol>
  );
}

export function Step({
  n,
  title,
  children,
}: {
  n: number;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <li className="relative">
      <span className="absolute -left-[2.45rem] flex h-7 w-7 items-center justify-center rounded-full border border-border bg-card font-mono text-2xs font-medium text-foreground">
        {n}
      </span>
      <h3 className="text-ms-17 font-semibold tracking-tight">{title}</h3>
      <div className="mt-2 text-ms-15 leading-relaxed text-muted-foreground [&_a]:font-medium [&_a]:text-primary hover:[&_a]:underline [&_code]:rounded [&_code]:bg-muted [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[0.85em] [&_code]:text-foreground">
        {children}
      </div>
    </li>
  );
}

export function FieldList({ children }: { children: React.ReactNode }) {
  return (
    <dl className="my-6 divide-y divide-border overflow-hidden rounded-xl border border-border">
      {children}
    </dl>
  );
}

export function Field({
  name,
  type,
  children,
}: {
  name: string;
  type?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="grid gap-1 px-5 py-4 sm:grid-cols-[12rem_1fr] sm:gap-6">
      <dt className="flex items-baseline gap-2">
        <code className="font-mono text-ms-13 text-foreground">{name}</code>
        {type && <span className="font-mono text-2xs text-muted-foreground">{type}</span>}
      </dt>
      <dd className="text-ms-15 leading-relaxed text-muted-foreground">{children}</dd>
    </div>
  );
}

export function Kbd({ children }: { children: React.ReactNode }) {
  return (
    <kbd className="inline-flex h-5 min-w-[1.25rem] items-center justify-center rounded border border-border bg-card px-1.5 font-mono text-2xs text-foreground shadow-sm">
      {children}
    </kbd>
  );
}

export function DocDivider() {
  return <hr className="my-10 border-border" />;
}

export function CardGrid({ children }: { children: React.ReactNode }) {
  return <div className="grid gap-4 sm:grid-cols-2">{children}</div>;
}

export function LinkCard({
  href,
  title,
  children,
  external,
}: {
  href: string;
  title: string;
  children: React.ReactNode;
  external?: boolean;
}) {
  return (
    <Link
      href={href}
      target={external ? "_blank" : undefined}
      rel={external ? "noreferrer" : undefined}
      className="group flex flex-col gap-2 rounded-xl border border-border bg-card p-5 shadow-sm transition-shadow hover:shadow-lift focus-ring"
    >
      <span className="flex items-center justify-between gap-2">
        <span className="text-ms-15 font-semibold tracking-tight text-foreground">
          {title}
        </span>
        <ArrowUpRight className="h-4 w-4 text-muted-foreground transition-transform group-hover:translate-x-0.5 group-hover:-translate-y-0.5" />
      </span>
      <span className="text-ms-13 leading-relaxed text-muted-foreground">{children}</span>
    </Link>
  );
}
