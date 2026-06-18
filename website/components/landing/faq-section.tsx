import Link from "next/link";

import { Section, SectionHeading } from "@/components/site/section";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";

const faqs = [
  {
    question: "Does my audio ever leave my machine?",
    answer:
      "Not on the default path. Capture, transcription, and diarization all run on-device. The only network calls are the one-time model download and the opt-in cloud-AI and webhook paths, all of which Privacy Mode can block.",
  },
  {
    question: "Does it join my calls as a bot?",
    answer:
      "No. Folio captures system audio directly through ScreenCaptureKit, so there is no bot in the meeting and nothing for other participants to admit.",
  },
  {
    question: "Where do my notes go?",
    answer:
      "Each meeting becomes a markdown file in the vault path you choose, with frontmatter for attendees, duration, model, and source. They are plain files you can read, edit, and back up however you like.",
  },
  {
    question: "Do I need an account or API key?",
    answer:
      "No account, ever. Local transcription works out of the box. An OpenAI key is only needed if you opt into cloud transcription or chat features.",
  },
  {
    question: "Which Macs are supported?",
    answer:
      "macOS 13 Ventura or later, on Apple Silicon or Intel. Apple Silicon is the performance target and gets Metal-accelerated Whisper.",
  },
];

export function FaqSection() {
  return (
    <Section className="bg-secondary/40">
      <div className="mx-auto max-w-3xl">
        <SectionHeading
          align="center"
          eyebrow="Questions"
          title="Answers before you install"
        />
        <Accordion type="single" collapsible className="mt-10">
          {faqs.map((faq) => (
            <AccordionItem key={faq.question} value={faq.question}>
              <AccordionTrigger>{faq.question}</AccordionTrigger>
              <AccordionContent>{faq.answer}</AccordionContent>
            </AccordionItem>
          ))}
        </Accordion>
        <p className="mt-8 text-center text-ms-15 text-muted-foreground">
          More in the{" "}
          <Link href="/docs/faq" className="text-primary underline-offset-2 hover:underline">
            full FAQ
          </Link>{" "}
          and the{" "}
          <Link href="/docs" className="text-primary underline-offset-2 hover:underline">
            documentation
          </Link>
          .
        </p>
      </div>
    </Section>
  );
}
