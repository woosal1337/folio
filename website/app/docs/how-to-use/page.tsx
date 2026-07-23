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
    title: "How to use",
    description:
        "Go from first launch to a searchable vault of meeting notes with Folio, the local-first meeting transcription app for macOS.",
};

const exampleFrontmatter = `---
attendees: [You, Speaker 1, Speaker 2]
duration: 47m
model: whisper.cpp
source: system + microphone
---`;

export default function HowToUsePage() {
    return (
        <>
            <DocHeader
                eyebrow="Getting started"
                title="Using Folio"
                description="Folio runs from the menu bar, records each meeting on your machine, and writes one markdown note per meeting to a vault you control. This page walks from first launch to a vault of notes you can search, edit, and trust."
            />

            <DocH2 id="first-launch">First launch</DocH2>
            <Prose>
                <p>
                    {siteConfig.name} is a menu-bar app. After you install it, it lives in
                    the macOS menu bar rather than the Dock, and it watches your calendar
                    and audio devices through EventKit. There is no window to keep open
                    and no bot that joins your calls.
                </p>
                <p>
                    On the first run, macOS prompts for two permissions. It asks for{" "}
                    <strong>microphone</strong> access so {siteConfig.name} can record
                    your voice, and for <strong>screen recording</strong> access, which is
                    required to capture system audio from the other participants. Grant
                    both. Without screen recording permission, the system-audio stream
                    cannot be captured.
                </p>
                <p>
                    {siteConfig.name} also downloads its model weights once on first use,
                    the first time it transcribes or diarizes locally. After that the
                    on-device path works without a network connection.
                </p>
                <p>
                    Finally, choose a <strong>vault path</strong>. This is the folder
                    where {siteConfig.name} writes one markdown file per meeting. Point it
                    at a plain directory, an existing notes folder, or anywhere you
                    already back up. The files are ordinary markdown, so any editor or
                    sync tool can read them.
                </p>
            </Prose>
            <Callout variant="privacy" title="Consent is your responsibility">
                <p>
                    Recording a conversation can be illegal without the other participants
                    consent. The rules vary by US state and by country, and many require
                    all-party consent. {siteConfig.name} gives you the tool. Obtaining
                    consent is on you. Tell people before you record.
                </p>
            </Callout>

            <DocDivider />

            <DocH2 id="recording-a-meeting">Recording a meeting</DocH2>
            <Prose>
                <p>
                    {siteConfig.name} captures two independent streams for every meeting.
                    One is the system audio of everyone else on the call. The other is
                    your microphone. Keeping them separate is what makes accurate speaker
                    labels possible later.
                </p>
            </Prose>
            <Steps>
                <Step n={1} title="Folio notices the meeting">
                    <p>
                        {siteConfig.name} watches your calendar and audio devices through
                        EventKit. When a meeting begins, it has the context it needs to
                        start capturing. No meeting bot joins the call and no service is
                        invited.
                    </p>
                </Step>
                <Step n={2} title="System audio and microphone are recorded separately">
                    <p>
                        ScreenCaptureKit captures the system audio, and <code>cpal</code>{" "}
                        captures your microphone. The two are recorded as distinct streams
                        rather than one mixed track. <code>rubato</code> handles
                        resampling and <code>hound</code> writes the WAV files.
                    </p>
                </Step>
                <Step n={3} title="Audio stays on your machine">
                    <p>
                        On the default path, the recordings never leave your Mac. Local
                        transcription with whisper.cpp and on-device diarization run
                        against the captured streams without any upload.
                    </p>
                </Step>
                <Step n={4} title="The note is written">
                    <p>
                        When the meeting ends, {siteConfig.name} writes a single markdown
                        file into your vault path. Your microphone track is always
                        labelled <code>You</code>, and the other voices are clustered into{" "}
                        <code>Speaker 1</code>, <code>Speaker 2</code>, and so on.
                    </p>
                </Step>
            </Steps>

            <DocDivider />

            <DocH2 id="the-note">The note</DocH2>
            <Prose>
                <p>
                    Each meeting becomes one markdown file you can read, edit, search, and
                    back up with any tool. The file opens with YAML frontmatter, followed
                    by the speaker-labelled transcript and the decisions and tasks drawn
                    from it.
                </p>
            </Prose>
            <DocH3 id="frontmatter">Frontmatter</DocH3>
            <Prose>
                <p>
                    The frontmatter block records the structured facts about the meeting.
                </p>
            </Prose>
            <FieldList>
                <Field name="attendees">
                    The people on the call, including <code>You</code> for your microphone
                    track and the clustered speakers.
                </Field>
                <Field name="duration">How long the meeting ran.</Field>
                <Field name="model">
                    The transcription model that produced the text.
                </Field>
                <Field name="source">Where the audio came from.</Field>
            </FieldList>
            <CodeBlock code={exampleFrontmatter} label="note.md" />
            <DocH3 id="transcript-and-tasks">Transcript, decisions, and tasks</DocH3>
            <Prose>
                <p>
                    Below the frontmatter sits the speaker-labelled transcript. Your own
                    microphone is always shown as <code>You</code>, and the system-audio
                    voices appear as <code>Speaker 1</code>, <code>Speaker 2</code>, and
                    so on. {siteConfig.name} also surfaces the decisions and tasks from
                    the conversation so the note is more than a wall of text.
                </p>
                <p>
                    Because the file is plain markdown, you stay in control. Rename a
                    speaker, fix a word, add a heading, or move the file. The note is
                    yours to keep.
                </p>
            </Prose>

            <DocDivider />

            <DocH2 id="reviewing-and-searching">Reviewing and searching</DocH2>
            <Prose>
                <p>
                    Notes accumulate into a vault you can navigate from inside{" "}
                    {siteConfig.name}. You can pull up past meetings, find the tasks that
                    came out of them, and search across the memory built from your notes.
                </p>
                <p>
                    The on-disk markdown files are the source of truth. {siteConfig.name}{" "}
                    uses a two-phase write, where the canonical file is written first and
                    the derived index second. The index is always rebuildable from the
                    files, so nothing is locked inside a database you cannot read.
                </p>
                <p>
                    The same data is also reachable from MCP-aware tools through the local{" "}
                    <code>folio-mcp</code> server, which gives read-only access to your
                    transcripts, tasks, and memories over stdio. See{" "}
                    <Link href="/docs/connectors">Connectors</Link> for how to wire it up.
                </p>
            </Prose>

            <DocDivider />

            <DocH2 id="cloud-transcription">Cloud transcription</DocH2>
            <Prose>
                <p>
                    The default transcription path is local. {siteConfig.name} bundles
                    whisper.cpp through the whisper-rs bindings, Metal-accelerated on
                    Apple Silicon, and this is the primary path for every meeting.
                </p>
                <p>
                    For long meetings, the OpenAI Whisper API is available as an opt-in
                    fallback for faster cloud transcription. It needs an OpenAI key, and
                    you turn it on yourself. It is never the default, and the local path
                    keeps working without it.
                </p>
            </Prose>
            <Callout variant="note" title="Opt-in only">
                <p>
                    Cloud transcription sends audio to OpenAI, so it is a deliberate
                    choice rather than a default. Leave it off and everything stays on
                    your machine. Turn it on only when the speed of a cloud run is worth
                    it for a long recording.
                </p>
            </Callout>

            <DocDivider />

            <DocH2 id="privacy-mode">Privacy Mode</DocH2>
            <Prose>
                <p>
                    When you want {siteConfig.name} fully airgapped, turn on Privacy Mode
                    under <strong>Settings</strong> then <strong>Privacy</strong>. It
                    physically blocks every outbound HTTP call except localhost. The app
                    keeps working end to end with Wi-Fi off.
                </p>
                <p>
                    {siteConfig.name} makes very few network calls to begin with. A
                    one-time model-weights download happens on first local transcription
                    or diarization, and the opt-in cloud-AI and webhook paths are the only
                    others. All of them can be blocked. Notes are also encrypted at rest
                    with AES-256-GCM and Argon2id, and {siteConfig.name} ships no
                    telemetry, analytics, or crash reporting.
                </p>
                <p>
                    For the full picture of what stays local and what crosses the network,
                    read the <Link href="/docs/privacy">Privacy</Link> page.
                </p>
            </Prose>
            <Callout variant="tip" title="Test it with the network off">
                <p>
                    Toggle Privacy Mode, turn off Wi-Fi, and record a meeting. The
                    capture, transcription, and diarization all run on-device, so the note
                    still lands in your vault.
                </p>
            </Callout>

            <DocDivider />

            <DocH2 id="next-steps">Next steps</DocH2>
            <Prose>
                <p>
                    If you have not installed {siteConfig.name} yet, start with the
                    install guide. To connect your notes to an MCP-aware tool, head to
                    Connectors.
                </p>
            </Prose>
            <CommandLine command={siteConfig.install.installCommand} />
            <CardGrid>
                <LinkCard href="/docs/installation" title="Install Folio">
                    Set up {siteConfig.name} on {siteConfig.platform} with Homebrew or a
                    direct download.
                </LinkCard>
                <LinkCard href="/docs/connectors" title="Connectors (MCP)">
                    Give Claude Desktop, Cursor, or Claude Code read-only access through{" "}
                    <code>folio-mcp</code>.
                </LinkCard>
                <LinkCard href="/docs/privacy" title="Privacy">
                    See exactly what stays on your machine and what never leaves it.
                </LinkCard>
                <LinkCard href={siteConfig.links.github} title="View the source" external>
                    {siteConfig.name} is {siteConfig.license} licensed and open on GitHub.
                </LinkCard>
            </CardGrid>
            <Prose>
                <p>
                    Hit a snag. Open an issue on{" "}
                    <a href={siteConfig.links.issues} target="_blank" rel="noreferrer">
                        GitHub
                    </a>{" "}
                    or press <Kbd>?</Kbd> inside the app.
                </p>
            </Prose>
        </>
    );
}
