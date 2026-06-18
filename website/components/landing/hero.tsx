import Link from "next/link";
import { ArrowRight, ShieldCheck } from "lucide-react";

import { siteConfig } from "@/lib/site-config";
import { Button } from "@/components/ui/button";
import { CommandLine } from "@/components/site/code-block";
import { NotePreview } from "@/components/landing/note-preview";

export function Hero() {
  return (
    <section className="relative overflow-hidden">
      <div
        className="pointer-events-none absolute inset-0 -z-10"
        aria-hidden
        style={{
          background:
            "radial-gradient(60% 50% at 50% 0%, hsl(var(--accent) / 0.6) 0%, transparent 70%)",
        }}
      />
      <div className="container grid items-center gap-16 py-20 sm:py-28 lg:grid-cols-[1.05fr_0.95fr] lg:gap-12">
        <div className="flex flex-col items-start gap-7">
          <span className="inline-flex items-center gap-2 rounded-full border border-border bg-card px-3 py-1 text-2xs font-medium text-muted-foreground shadow-sm">
            <ShieldCheck className="h-3.5 w-3.5 text-primary" />
            Local-first · No telemetry · {siteConfig.platform}
          </span>

          <h1 className="text-balance font-display text-ms-45 font-semibold leading-[1.04] tracking-tight sm:text-ms-57 lg:text-ms-72">
            Meeting notes that never leave your Mac.
          </h1>

          <p className="max-w-xl text-pretty text-ms-17 leading-relaxed text-muted-foreground sm:text-ms-22">
            {siteConfig.name} captures system audio and your microphone, transcribes
            on-device, and writes a clean markdown note per meeting to your own vault.
            Private by default.
          </p>

          <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
            <Button asChild size="lg">
              <Link href="/docs/installation">
                Install {siteConfig.name}
                <ArrowRight className="h-4 w-4" />
              </Link>
            </Button>
            <Button asChild size="lg" variant="outline">
              <Link href="/docs">Read the docs</Link>
            </Button>
          </div>

          <CommandLine
            command={siteConfig.install.installCommand}
            className="w-full max-w-md"
          />
        </div>

        <div className="relative flex justify-center lg:justify-end">
          <div
            className="pointer-events-none absolute -inset-6 -z-10 rounded-[2rem] opacity-70"
            aria-hidden
            style={{
              background:
                "radial-gradient(70% 70% at 60% 30%, hsl(var(--signal) / 0.18) 0%, transparent 70%)",
            }}
          />
          <NotePreview className="motion-safe:animate-fade-up" />
        </div>
      </div>
    </section>
  );
}
