import { CalendarClock, Mic, ScrollText, Sparkles } from "lucide-react";

import { Section, SectionHeading } from "@/components/site/section";

const steps = [
    {
        icon: CalendarClock,
        step: "01",
        title: "It notices the meeting",
        description:
            "Folio watches your calendar and audio devices through EventKit. No Google OAuth, no Microsoft Graph, no meeting bots.",
    },
    {
        icon: Mic,
        step: "02",
        title: "It records both sides",
        description:
            "When the call starts, system audio and your microphone are captured as separate streams and written to WAV on disk.",
    },
    {
        icon: ScrollText,
        step: "03",
        title: "It transcribes locally",
        description:
            "The bundled Whisper backend transcribes on-device and diarizes speakers. Cloud transcription is an explicit opt-in, never the default.",
    },
    {
        icon: Sparkles,
        step: "04",
        title: "It writes the note",
        description:
            "A markdown file lands in your vault with frontmatter, speaker-labelled transcript, decisions, and tasks. Searchable forever.",
    },
];

export function HowItWorks() {
    return (
        <Section className="bg-secondary/40">
            <SectionHeading
                eyebrow="How it works"
                title="From a live call to a note you own"
                description="Four steps, all on your machine. The only time anything leaves is the one-time model download or an opt-in cloud call you trigger yourself."
            />

            <ol className="mt-14 grid gap-4 md:grid-cols-2 lg:grid-cols-4">
                {steps.map((item) => (
                    <li
                        key={item.step}
                        className="relative flex flex-col gap-4 rounded-xl border border-border bg-card p-6 shadow-sm"
                    >
                        <div className="flex items-center justify-between">
                            <span className="inline-flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
                                <item.icon className="h-5 w-5" />
                            </span>
                            <span className="font-mono text-2xs tracking-[0.18em] text-muted-foreground">
                                {item.step}
                            </span>
                        </div>
                        <h3 className="text-ms-17 font-semibold tracking-tight">
                            {item.title}
                        </h3>
                        <p className="text-ms-15 leading-relaxed text-muted-foreground">
                            {item.description}
                        </p>
                    </li>
                ))}
            </ol>
        </Section>
    );
}
