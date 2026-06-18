import * as React from "react";

import { cn } from "@/lib/utils";

export function Section({
  className,
  containerClassName,
  children,
  ...props
}: React.HTMLAttributes<HTMLElement> & { containerClassName?: string }) {
  return (
    <section className={cn("py-20 sm:py-28", className)} {...props}>
      <div className={cn("container", containerClassName)}>{children}</div>
    </section>
  );
}

export function Eyebrow({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-2 font-mono text-xs uppercase tracking-[0.18em] text-muted-foreground",
        className
      )}
    >
      <span className="h-1.5 w-1.5 rounded-full bg-foreground/40" aria-hidden />
      {children}
    </span>
  );
}

export function SectionHeading({
  eyebrow,
  title,
  description,
  align = "left",
  className,
}: {
  eyebrow?: string;
  title: React.ReactNode;
  description?: React.ReactNode;
  align?: "left" | "center";
  className?: string;
}) {
  return (
    <div
      className={cn(
        "flex flex-col gap-4",
        align === "center" && "mx-auto max-w-2xl text-center",
        className
      )}
    >
      {eyebrow && (
        <Eyebrow className={cn(align === "center" && "justify-center")}>{eyebrow}</Eyebrow>
      )}
      <h2 className="text-balance font-display text-ms-34 font-semibold tracking-tight sm:text-ms-45">
        {title}
      </h2>
      {description && (
        <p className="text-pretty text-ms-17 leading-relaxed text-muted-foreground">
          {description}
        </p>
      )}
    </div>
  );
}
