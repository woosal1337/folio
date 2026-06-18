import Link from "next/link";
import { ArrowRight, Check, WifiOff } from "lucide-react";

import { siteConfig } from "@/lib/site-config";
import { Section, SectionHeading } from "@/components/site/section";
import { Button } from "@/components/ui/button";

const guarantees = [
  "No telemetry, no analytics, no crash reporting — enforced in CI.",
  "Audio, transcripts, and notes stay on disk on the default path.",
  "Privacy Mode blocks every outbound call except localhost.",
  "Notes are encrypted at rest with AES-256-GCM and Argon2id.",
];

const egress = [
  { label: "Microphone capture", status: "local", blocked: false },
  { label: "System audio capture", status: "local", blocked: false },
  { label: "Whisper transcription", status: "local", blocked: false },
  { label: "Speaker diarization", status: "local", blocked: false },
  { label: "Outbound HTTP", status: "blocked", blocked: true },
];

export function PrivacySection() {
  return (
    <Section>
      <div className="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
        <div className="flex flex-col gap-6">
          <SectionHeading
            eyebrow="Privacy"
            title="The network is opt-in, not the default"
            description="Folio is built so the private path is the easy path. You can run an entire meeting end-to-end with Wi-Fi off."
          />
          <ul className="flex flex-col gap-3">
            {guarantees.map((item) => (
              <li key={item} className="flex items-start gap-3">
                <span className="mt-0.5 inline-flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 text-primary">
                  <Check className="h-3 w-3" />
                </span>
                <span className="text-ms-15 leading-relaxed text-muted-foreground">
                  {item}
                </span>
              </li>
            ))}
          </ul>
          <Button asChild variant="outline" className="w-fit">
            <Link href="/docs/privacy">
              How privacy works
              <ArrowRight className="h-4 w-4" />
            </Link>
          </Button>
        </div>

        <div className="rounded-2xl border border-border bg-card p-6 shadow-lift">
          <div className="flex items-center justify-between border-b border-border pb-4">
            <span className="font-mono text-2xs uppercase tracking-[0.16em] text-muted-foreground">
              Privacy Mode
            </span>
            <span className="inline-flex items-center gap-1.5 rounded-full bg-accent px-2.5 py-1 font-mono text-2xs text-accent-foreground">
              <WifiOff className="h-3 w-3" />
              airgapped
            </span>
          </div>
          <ul className="flex flex-col divide-y divide-border">
            {egress.map((row) => (
              <li key={row.label} className="flex items-center justify-between py-3.5">
                <span className="text-ms-15">{row.label}</span>
                <span
                  className={
                    row.blocked
                      ? "inline-flex items-center gap-1.5 rounded-full bg-muted px-2.5 py-0.5 font-mono text-2xs text-muted-foreground line-through decoration-muted-foreground/50"
                      : "inline-flex items-center gap-1.5 rounded-full bg-foreground/10 px-2.5 py-0.5 font-mono text-2xs text-foreground"
                  }
                >
                  {row.status}
                </span>
              </li>
            ))}
          </ul>
          <p className="mt-4 text-2xs leading-relaxed text-muted-foreground">
            Full details, retention, and the recording-consent guide live in the{" "}
            <Link href="/docs/privacy" className="text-primary underline-offset-2 hover:underline">
              privacy documentation
            </Link>
            . {siteConfig.name} gives you the tool; obtaining consent is your responsibility.
          </p>
        </div>
      </div>
    </Section>
  );
}
