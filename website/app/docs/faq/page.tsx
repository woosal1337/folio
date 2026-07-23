import type { Metadata } from "next";
import Link from "next/link";
import { siteConfig } from "@/lib/site-config";
import { DocHeader, DocH2, DocH3, Prose } from "@/components/docs/doc-primitives";

export const metadata: Metadata = {
    title: "FAQ",
    description:
        "Short answers to common questions about how Folio captures meetings, where your data lives, and which Macs it runs on.",
};

export default function FaqPage() {
    return (
        <>
            <DocHeader
                eyebrow="Reference"
                title="Frequently asked questions"
                description={`Short answers to common questions about ${siteConfig.name}. How it captures meetings, where your notes live, and what runs on your machine.`}
            />

            <DocH2 id="getting-started">Getting started</DocH2>

            <DocH3 id="does-it-join-my-calls-as-a-bot">
                Does it join my calls as a bot?
            </DocH3>
            <Prose>
                <p>
                    No. {siteConfig.name} never joins your call. It is a menu-bar app that
                    captures the system audio on your Mac through ScreenCaptureKit, so it
                    works with any meeting tool without a participant showing up in the
                    call. There is no bot, no proxy, and no third-party service sitting
                    between you and the people you are talking to.
                </p>
            </Prose>

            <DocH3 id="do-i-need-an-account-or-api-key">
                Do I need an account or API key?
            </DocH3>
            <Prose>
                <p>
                    No account is required. {siteConfig.name} works out of the box with
                    on-device transcription, so the local path needs no key and no
                    sign-up. An <code>OpenAI</code> key is only needed if you opt in to
                    the cloud transcription fallback. That fallback is never the default.
                </p>
            </Prose>

            <DocH3 id="how-does-it-know-a-meeting-started">
                How does it know a meeting started?
            </DocH3>
            <Prose>
                <p>
                    {siteConfig.name} watches your calendar and your audio devices through
                    EventKit. When a meeting starts it records the system audio and your
                    microphone as two separate streams. There is no Google OAuth and no
                    Microsoft Graph involved.
                </p>
            </Prose>

            <DocH2 id="privacy-and-data">Privacy and data</DocH2>

            <DocH3 id="does-my-audio-ever-leave-my-machine">
                Does my audio ever leave my machine?
            </DocH3>
            <Prose>
                <p>
                    Not on the default path. Audio, transcripts, and notes stay on your
                    Mac. The only network calls {siteConfig.name} ever makes are a
                    one-time model-weights download on first use and the opt-in cloud and
                    webhook paths, all of which can be blocked. See the{" "}
                    <Link href="/docs/privacy">privacy page</Link> for the full picture.
                </p>
            </Prose>

            <DocH3 id="where-do-my-notes-go">Where do my notes go?</DocH3>
            <Prose>
                <p>
                    {siteConfig.name} writes one markdown file per meeting into a vault
                    path you choose. Each file is plain markdown you can read, edit,
                    search, and back up with any tool. The frontmatter includes attendees,
                    duration, model, and source.
                </p>
            </Prose>

            <DocH3 id="can-i-run-it-offline">Can I run it offline?</DocH3>
            <Prose>
                <p>
                    Yes. Privacy Mode, under <code>Settings</code> then{" "}
                    <code>Privacy</code>, physically blocks every outbound HTTP call
                    except localhost. After the model weights are downloaded once, the app
                    keeps working end to end with Wi-Fi off.
                </p>
            </Prose>

            <DocH3 id="is-anything-encrypted">Is anything encrypted?</DocH3>
            <Prose>
                <p>
                    Yes. Notes are encrypted at rest with <code>AES-256-GCM</code> and{" "}
                    <code>Argon2id</code>. There is no telemetry, no analytics, and no
                    crash reporting, and that absence is enforced in CI by a dedicated
                    no-telemetry job.
                </p>
            </Prose>

            <DocH2 id="platform-and-pricing">Platform and pricing</DocH2>

            <DocH3 id="which-macs-are-supported">Which Macs are supported?</DocH3>
            <Prose>
                <p>
                    {siteConfig.name} runs on {siteConfig.platform}, on Apple Silicon or
                    Intel. Apple Silicon is the performance target, where on-device
                    transcription is Metal-accelerated. Intel users build from source.
                </p>
            </Prose>

            <DocH3 id="is-it-free-and-open-source">Is it free and open source?</DocH3>
            <Prose>
                <p>
                    Yes. {siteConfig.name} is open source under the {siteConfig.license}{" "}
                    license. You can read the code, the{" "}
                    <a href={siteConfig.links.license} target="_blank" rel="noreferrer">
                        license
                    </a>
                    , and the full project on{" "}
                    <a href={siteConfig.links.github} target="_blank" rel="noreferrer">
                        GitHub
                    </a>
                    .
                </p>
            </Prose>

            <DocH3 id="is-there-a-windows-or-linux-version">
                Is there a Windows or Linux version?
            </DocH3>
            <Prose>
                <p>
                    No. {siteConfig.name} is macOS only. It depends on macOS frameworks
                    such as ScreenCaptureKit and EventKit for capture and meeting
                    detection, so there is no Windows or Linux build.
                </p>
            </Prose>

            <DocH3 id="how-do-i-get-speaker-names">How do I get speaker names?</DocH3>
            <Prose>
                <p>
                    {siteConfig.name} runs diarization on device, on the system-audio
                    track, and clusters the voices into <code>Speaker 1</code>,{" "}
                    <code>Speaker 2</code>, <code>Speaker 3</code>, and so on. Your own
                    microphone track is always labelled <code>You</code>. No cloud is
                    involved in diarization.
                </p>
            </Prose>

            <Prose>
                <p>
                    Still stuck? Start from the{" "}
                    <Link href="/docs">documentation home</Link>, or open a question on{" "}
                    <a href={siteConfig.links.issues} target="_blank" rel="noreferrer">
                        GitHub issues
                    </a>
                    .
                </p>
            </Prose>
        </>
    );
}
