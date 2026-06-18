import type { Metadata } from "next";
import Link from "next/link";
import { siteConfig } from "@/lib/site-config";
import { CodeBlock, CommandLine } from "@/components/site/code-block";
import {
  DocHeader,
  DocH2,
  DocH3,
  Prose,
  Callout,
  Steps,
  Step,
  FieldList,
  Field,
  Kbd,
  DocDivider,
  CardGrid,
  LinkCard,
} from "@/components/docs/doc-primitives";

export const metadata: Metadata = {
  title: "Privacy & consent",
  description:
    "How the private path is the default path in Folio, what stays on disk, the exact network surface, and your responsibility around recording consent.",
};

export default function PrivacyPage() {
  return (
    <>
      <DocHeader
        eyebrow="Going deeper"
        title="Privacy and consent"
        description="The private path is the default path. On a fresh install, your audio, transcripts, and notes stay on your machine. You do not turn privacy on. You turn it off, deliberately, only when you choose to use an opt-in path."
      />

      <DocH2 id="what-stays-local">What stays local</DocH2>
      <Prose>
        <p>
          {siteConfig.name} is a local-first application. It captures system
          audio and your microphone, transcribes on-device, and writes one
          markdown note per meeting to a vault path you choose. On the default
          path, none of that leaves your machine.
        </p>
        <p>The guarantees that hold by default:</p>
        <ul>
          <li>
            <strong>No telemetry, no analytics, no crash reporting.</strong>{" "}
            This is not a setting you trust us to honor. It is enforced in CI by
            a dedicated <code>no-telemetry</code> job that runs on every push and
            pull request to main.
          </li>
          <li>
            <strong>Audio, transcripts, and notes stay on disk.</strong> The
            recorded streams, the on-device transcription output, and every
            markdown note are written to your machine and read from your
            machine.
          </li>
          <li>
            <strong>Notes are encrypted at rest.</strong> {siteConfig.name}{" "}
            encrypts notes with <code>AES-256-GCM</code>, with key derivation
            through <code>Argon2id</code>.
          </li>
        </ul>
        <p>
          Local transcription runs through <code>whisper.cpp</code> on-device,
          and diarization runs on-device as well. Neither path sends audio to a
          server.
        </p>
      </Prose>

      <DocH2 id="the-network-surface">The network surface</DocH2>
      <Prose>
        <p>
          It is easier to trust a tool when you can name every call it is able
          to make. {siteConfig.name} makes only these network calls, and each one
          can be blocked.
        </p>
        <ul>
          <li>
            <strong>A one-time model-weights download.</strong> On first local
            transcription or diarization, {siteConfig.name} downloads the model
            weights once. This is a few hundred megabytes, pulled from Hugging
            Face and from the <code>sherpa-onnx</code> GitHub releases. After
            that, the local path needs no network at all.
          </li>
          <li>
            <strong>The opt-in cloud-AI path.</strong> The OpenAI Whisper API is
            an opt-in fallback for faster cloud transcription on long meetings.
            It needs an OpenAI key. It is never the default, and you choose when
            to use it.
          </li>
          <li>
            <strong>The opt-in webhook path.</strong> If you wire up a webhook,
            {" "}
            {siteConfig.name} can call it. This is opt-in and off until you set
            it up.
          </li>
        </ul>
        <p>
          Every item in that list can be blocked. If you never enable the
          cloud-AI path or a webhook, and you keep the model weights you already
          downloaded, {siteConfig.name} makes no outbound calls.
        </p>
      </Prose>

      <DocH2 id="privacy-mode">Privacy Mode</DocH2>
      <Prose>
        <p>
          When you want a hard guarantee instead of a careful configuration,
          turn on Privacy Mode under <strong>Settings</strong> then{" "}
          <strong>Privacy</strong>. Privacy Mode physically blocks every outbound
          HTTP call except <code>localhost</code>. There is nothing left to
          forget about and no path to leak through.
        </p>
        <p>
          The proof is that the app keeps working. With Privacy Mode on, you can
          turn Wi-Fi off and {siteConfig.name} still records, transcribes,
          diarizes, and writes notes end to end. The local pipeline does not
          depend on the network, so removing the network changes nothing about
          the result.
        </p>
        <p>
          The one prerequisite is that the model weights are already on disk. If
          you have run a local transcription or diarization once before, the
          one-time download has already happened and Privacy Mode has nothing it
          needs to reach for.
        </p>
      </Prose>

      <DocH2 id="data-retention">Data retention</DocH2>
      <Prose>
        <p>
          {siteConfig.name} does not hold your data in a place you cannot see.
          Notes are plain markdown files in the vault path you chose. You can
          read them, edit them, search them, and back them up with any tool you
          already use.
        </p>
        <p>
          Retention is therefore in your hands. A note exists for exactly as long
          as the file exists. When you delete the file, the note is gone. There
          is no separate copy to clear and no server-side record to request the
          deletion of. The on-disk file is the canonical artifact, and the
          derived index is always rebuildable from the files.
        </p>
        <p>
          For the full account of what {siteConfig.name} stores and what it never
          touches, read the{" "}
          <a href={siteConfig.links.privacy} target="_blank" rel="noreferrer">
            privacy document
          </a>
          .
        </p>
      </Prose>

      <DocH2 id="recording-consent">Recording consent</DocH2>
      <Prose>
        <p>
          {siteConfig.name} gives you a recording tool. Using it lawfully is your
          responsibility, and the law here is not uniform.
        </p>
      </Prose>
      <Callout variant="privacy" title="Consent is your responsibility">
        <p>
          Recording a conversation can be illegal without the other participants
          consent. The rules vary by US state and by country, and many require
          all-party consent. {siteConfig.name} gives you the tool. Obtaining
          consent is your responsibility. Tell people before you record.
        </p>
      </Callout>

      <DocDivider />

      <Prose>
        <p>
          For the complete and authoritative account of {siteConfig.name}{" "}
          privacy, including the full list of what stays on your machine and what
          the opt-in paths do, read the{" "}
          <a href={siteConfig.links.privacy} target="_blank" rel="noreferrer">
            full privacy document
          </a>
          .
        </p>
      </Prose>
    </>
  );
}
