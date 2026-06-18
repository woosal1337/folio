import type { Metadata } from "next";
import Link from "next/link";
import {
  AudioLines,
  Boxes,
  BrainCircuit,
  Check,
  FileText,
  Lock,
  Plug,
  Users,
} from "lucide-react";

import { siteConfig } from "@/lib/site-config";
import { cn } from "@/lib/utils";
import { Section, SectionHeading, Eyebrow } from "@/components/site/section";
import { Button } from "@/components/ui/button";
import { CommandLine } from "@/components/site/code-block";
import { NotePreview } from "@/components/landing/note-preview";
import { CapabilityTable } from "@/components/landing/capability-table";

export const metadata: Metadata = {
  title: "Features",
  description:
    "Everything Folio does, in detail: two-stream capture, on-device transcription and diarization, a markdown vault you own, local connectors, and a network you control.",
};

type FeatureBlock = {
  icon: typeof AudioLines;
  eyebrow: string;
  title: string;
  description: string;
  points: string[];
};

const blocks: FeatureBlock[] = [
  {
    icon: AudioLines,
    eyebrow: "Capture",
    title: "Two streams, kept apart",
    description:
      "Folio records system audio and your microphone as independent tracks, so the transcript stays clean and every line is attributable.",
    points: [
      "cpal captures your microphone, ScreenCaptureKit captures system audio.",
      "No bot joins the call, so there is nothing for other people to admit.",
      "It watches your calendar and audio devices through EventKit, with no OAuth.",
      "rubato resamples and hound writes WAV, entirely on disk.",
    ],
  },
  {
    icon: FileText,
    eyebrow: "Transcription",
    title: "On-device by default",
    description:
      "The bundled Whisper backend runs locally and is Metal-accelerated on Apple Silicon. Cloud transcription is an explicit choice, never the default.",
    points: [
      "whisper.cpp through whisper-rs handles local transcription.",
      "OpenAI Whisper is an opt-in fallback for faster runs on long meetings.",
      "Cloud calls require your own key and are gated behind a clear prompt.",
      "Privacy Mode keeps everything local, even on long recordings.",
    ],
  },
  {
    icon: Users,
    eyebrow: "Diarization",
    title: "Who said what, computed locally",
    description:
      "Speakers are separated on-device with pyannote segmentation and a speaker-embedding model, both run through sherpa-onnx.",
    points: [
      "Voices on the system track are clustered into Speaker 1, 2, 3 and so on.",
      "Your microphone is always labelled You.",
      "Nothing about your voiceprint leaves the machine.",
      "The result lands inline in the markdown transcript.",
    ],
  },
  {
    icon: BrainCircuit,
    eyebrow: "Your vault",
    title: "One markdown note per meeting",
    description:
      "Every conversation becomes a portable markdown file in a vault path you choose, with structured frontmatter and a clean transcript.",
    points: [
      "Frontmatter records attendees, duration, model, and source.",
      "A two-phase write keeps the file canonical and the index rebuildable.",
      "Decisions and tasks are pulled out so you can act on them.",
      "Plain files mean any editor, any backup, no lock-in.",
    ],
  },
  {
    icon: Plug,
    eyebrow: "Connectors",
    title: "Your agents can read your meetings",
    description:
      "A local MCP server exposes transcripts, tasks, and memory to MCP-aware tools over stdio, read-only, with no cloud in the middle.",
    points: [
      "Works with Claude Desktop, Cursor, Claude Code, and any MCP client.",
      "Search past meetings without leaving your editor.",
      "Pull decisions and action items into your own workflow.",
      "Scoped to read-only access over a local stdio connection.",
    ],
  },
  {
    icon: Lock,
    eyebrow: "Privacy",
    title: "The network is opt-in",
    description:
      "Folio is built so the private path is the easy path. Privacy Mode airgaps the app, and there is no telemetry to turn off.",
    points: [
      "No telemetry, no analytics, no crash reporting, enforced in CI.",
      "Privacy Mode blocks every outbound call except localhost.",
      "Notes are encrypted at rest with AES-256-GCM and Argon2id.",
      "The app runs end to end with Wi-Fi off.",
    ],
  },
];

function FeatureSplit({ block, index }: { block: FeatureBlock; index: number }) {
  const flipped = index % 2 === 1;
  return (
    <div className="grid items-center gap-10 lg:grid-cols-2 lg:gap-16">
      <div className={cn("flex flex-col gap-5", flipped && "lg:order-2")}>
        <Eyebrow>{block.eyebrow}</Eyebrow>
        <h3 className="text-balance font-display text-ms-28 font-semibold tracking-tight">
          {block.title}
        </h3>
        <p className="text-pretty text-ms-17 leading-relaxed text-muted-foreground">
          {block.description}
        </p>
        <ul className="flex flex-col gap-3">
          {block.points.map((point) => (
            <li key={point} className="flex items-start gap-3">
              <span className="mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
                <Check className="h-3 w-3" />
              </span>
              <span className="text-ms-15 leading-relaxed text-muted-foreground">
                {point}
              </span>
            </li>
          ))}
        </ul>
      </div>

      <div className={cn("flex justify-center", flipped && "lg:order-1")}>
        <div className="flex h-full w-full items-center justify-center rounded-2xl border border-border bg-secondary/40 p-10">
          <block.icon className="h-20 w-20 text-primary/80" strokeWidth={1.25} />
        </div>
      </div>
    </div>
  );
}

export default function FeaturesPage() {
  return (
    <>
      <section className="border-b border-border">
        <div className="container flex flex-col items-start gap-6 py-20 sm:py-24">
          <Eyebrow>Features</Eyebrow>
          <h1 className="max-w-3xl text-balance font-display text-ms-45 font-semibold leading-[1.05] tracking-tight sm:text-ms-57">
            Built to capture meetings without giving them away
          </h1>
          <p className="max-w-2xl text-pretty text-ms-17 leading-relaxed text-muted-foreground sm:text-ms-22">
            {siteConfig.name} does the obvious thing well. It records, transcribes, and
            files your meetings, and it does almost all of it on your own machine.
          </p>
          <div className="flex flex-col gap-3 sm:flex-row">
            <Button asChild size="lg">
              <Link href="/docs/installation">Install {siteConfig.name}</Link>
            </Button>
            <Button asChild size="lg" variant="outline">
              <Link href="/docs">Read the docs</Link>
            </Button>
          </div>
        </div>
      </section>

      <Section className="space-y-24 sm:space-y-32">
        {blocks.map((block, index) => (
          <FeatureSplit key={block.title} block={block} index={index} />
        ))}
      </Section>

      <Section className="bg-secondary/40">
        <SectionHeading
          align="center"
          eyebrow="What runs where"
          title="Local by default, cloud by choice"
          description="Every core capability runs on your machine. The cloud is only ever reached when you ask for it, and Privacy Mode can shut even that off."
        />
        <div className="mx-auto mt-12 max-w-2xl">
          <CapabilityTable />
        </div>
      </Section>

      <Section>
        <div className="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
          <div className="flex flex-col gap-6">
            <Eyebrow>The output</Eyebrow>
            <h2 className="text-balance font-display text-ms-34 font-semibold tracking-tight">
              Everything ends as a note you own
            </h2>
            <p className="text-pretty text-ms-17 leading-relaxed text-muted-foreground">
              No dashboard to log into, no export to request. Each meeting is a markdown
              file with frontmatter, a speaker-labelled transcript, and the decisions and
              tasks that came out of it.
            </p>
            <div className="flex items-center gap-3 text-ms-13 text-muted-foreground">
              <Boxes className="h-4 w-4 text-primary" />
              Plain files, in your vault, forever.
            </div>
            <CommandLine command={siteConfig.install.installCommand} className="max-w-sm" />
          </div>
          <div className="flex justify-center lg:justify-end">
            <NotePreview />
          </div>
        </div>
      </Section>
    </>
  );
}
