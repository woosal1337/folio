import { FileText, Hash, Lock } from "lucide-react";

import { cn } from "@/lib/utils";
import { WaveBars } from "@/components/landing/wave-bars";

const transcript = [
  { speaker: "You", tone: "primary", line: "Let's lock the launch date before we scope the rest." },
  { speaker: "Speaker 2", tone: "muted", line: "Works for me. I'll own the migration checklist." },
  { speaker: "Speaker 3", tone: "muted", line: "I can have the pricing page ready by Thursday." },
  { speaker: "You", tone: "primary", line: "Decision: ship on the 24th, freeze scope Friday." },
];

export function NotePreview({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        "w-full max-w-md overflow-hidden rounded-2xl border border-border bg-card shadow-lift",
        className
      )}
    >
      <div className="flex items-center justify-between border-b border-border bg-secondary/50 px-5 py-3">
        <div className="flex items-center gap-2 text-muted-foreground">
          <FileText className="h-4 w-4" />
          <span className="font-mono text-2xs tracking-wide">
            launch-sync-2026-06-18.md
          </span>
        </div>
        <span className="inline-flex items-center gap-1.5 rounded-full bg-accent px-2 py-0.5 font-mono text-2xs text-accent-foreground">
          <Lock className="h-3 w-3" />
          local
        </span>
      </div>

      <div className="space-y-1 border-b border-border px-5 py-4 font-mono text-2xs leading-relaxed text-muted-foreground">
        <p>
          <span className="text-primary">title</span>: Launch sync
        </p>
        <p>
          <span className="text-primary">attendees</span>: You, Speaker 2, Speaker 3
        </p>
        <p>
          <span className="text-primary">duration</span>: 28m · <span className="text-primary">model</span>: whisper-large-v3
        </p>
        <p>
          <span className="text-primary">source</span>: on-device
        </p>
      </div>

      <div className="space-y-3.5 px-5 py-5">
        {transcript.map((row, index) => (
          <div key={index} className="flex gap-3">
            <span
              className={cn(
                "mt-0.5 w-16 shrink-0 font-mono text-2xs",
                row.tone === "primary" ? "text-primary" : "text-muted-foreground"
              )}
            >
              {row.speaker}
            </span>
            <p className="text-ms-13 leading-relaxed text-foreground/90">{row.line}</p>
          </div>
        ))}
      </div>

      <div className="flex items-center justify-between border-t border-border bg-secondary/40 px-5 py-3">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Hash className="h-3.5 w-3.5" />
          <span className="font-mono text-2xs">1 decision · 2 tasks</span>
        </div>
        <WaveBars className="h-5 w-24 text-primary/70" />
      </div>
    </div>
  );
}
