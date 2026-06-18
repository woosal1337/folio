import {
  AudioLines,
  Braces,
  FileText,
  Plug,
  ShieldCheck,
  Users,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { Section, SectionHeading } from "@/components/site/section";
import { WaveBars } from "@/components/landing/wave-bars";

type Feature = {
  icon: typeof FileText;
  title: string;
  description: string;
  className?: string;
  accent?: boolean;
};

const features: Feature[] = [
  {
    icon: AudioLines,
    title: "Two streams, captured separately",
    description:
      "System audio and your microphone are recorded as independent tracks via cpal and ScreenCaptureKit, so the transcript stays clean and attributable.",
    className: "lg:col-span-2",
    accent: true,
  },
  {
    icon: ShieldCheck,
    title: "Private by default",
    description:
      "The default path keeps audio, transcripts, and notes on your machine. Privacy Mode physically blocks every outbound call except localhost.",
  },
  {
    icon: Users,
    title: "On-device diarization",
    description:
      "Speakers are separated locally with pyannote segmentation and a speaker-embedding model. Your microphone is always labelled You.",
  },
  {
    icon: FileText,
    title: "One markdown note per meeting",
    description:
      "Every meeting becomes a portable markdown file in your own vault, with frontmatter for attendees, duration, model, and source.",
    className: "lg:col-span-2",
  },
  {
    icon: Plug,
    title: "Connect your tools",
    description:
      "A local MCP server gives Claude, Cursor, and Claude Code read-only access to your transcripts, tasks, and memory over stdio.",
  },
  {
    icon: Braces,
    title: "Yours to own",
    description:
      "Plain files on disk, an open IPC contract, and an Apache-2.0 codebase. No lock-in, no account, no cloud dependency.",
    className: "lg:col-span-2",
  },
];

export function FeatureGrid() {
  return (
    <Section>
      <SectionHeading
        eyebrow="What it does"
        title="A meeting recorder that respects your machine"
        description="Folio sits in the menu bar, watches your calendar and audio devices, and turns each conversation into a note you actually own."
      />

      <div className="mt-14 grid gap-4 lg:grid-cols-3">
        {features.map((feature) => (
          <article
            key={feature.title}
            className={cn(
              "group relative flex flex-col gap-4 rounded-xl border border-border bg-card p-6 shadow-sm transition-shadow hover:shadow-lift",
              feature.className
            )}
          >
            <div className="flex items-center justify-between">
              <span className="inline-flex h-10 w-10 items-center justify-center rounded-lg bg-accent text-accent-foreground">
                <feature.icon className="h-5 w-5" />
              </span>
              {feature.accent && (
                <WaveBars className="h-6 w-20 text-primary/50" />
              )}
            </div>
            <h3 className="text-ms-17 font-semibold tracking-tight">{feature.title}</h3>
            <p className="text-ms-15 leading-relaxed text-muted-foreground">
              {feature.description}
            </p>
          </article>
        ))}
      </div>
    </Section>
  );
}
