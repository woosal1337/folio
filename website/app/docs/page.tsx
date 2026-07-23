import type { Metadata } from "next";
import Link from "next/link";

import { siteConfig } from "@/lib/site-config";
import {
    DocHeader,
    DocH2,
    Prose,
    Callout,
    CardGrid,
    LinkCard,
} from "@/components/docs/doc-primitives";

export const metadata: Metadata = {
    title: "Documentation",
    description:
        "Get started with Folio, the local-first app that captures, transcribes, and files your meetings on your own machine.",
};

export default function DocsOverviewPage() {
    return (
        <>
            <DocHeader
                eyebrow="Documentation"
                title="Folio documentation"
                description={`${siteConfig.name} captures your meetings, transcribes them on-device, and files one markdown note per meeting into a vault you control. Everything happens locally.`}
            />

            <Callout variant="note" title="Local by default">
                <Prose>
                    <p>
                        On the default path, audio, transcripts, and notes stay on your
                        machine. Privacy Mode, under <strong>Settings</strong> then{" "}
                        <strong>Privacy</strong>, blocks every outbound HTTP call except{" "}
                        <code>localhost</code>, so you can airgap the app entirely and it
                        still works end to end.
                    </p>
                </Prose>
            </Callout>

            <DocH2 id="what-is-folio">What is Folio</DocH2>
            <Prose>
                <p>
                    <strong>{siteConfig.name}</strong> is a menu-bar app for{" "}
                    {siteConfig.platform}. When a meeting starts it records your system
                    audio and your microphone as two independent streams, transcribes them
                    on-device, and writes one markdown note per meeting into a vault path
                    you choose. No meeting bot joins the call. There is no Google OAuth
                    and no Microsoft Graph. {siteConfig.name} watches your calendar and
                    audio devices locally through EventKit.
                </p>
                <p>
                    The promise is simple. On the default path your audio never leaves
                    your machine. Local transcription with <code>whisper.cpp</code> is the
                    primary path, on-device diarization separates voices, and the notes
                    are plain markdown files you can read, edit, search, and back up with
                    any tool. {siteConfig.name} is{" "}
                    <a href={siteConfig.links.license} target="_blank" rel="noreferrer">
                        {siteConfig.license}
                    </a>{" "}
                    licensed and open source on{" "}
                    <a href={siteConfig.links.github} target="_blank" rel="noreferrer">
                        GitHub
                    </a>
                    .
                </p>
            </Prose>

            <DocH2 id="core-ideas">Core ideas</DocH2>
            <Prose>
                <p>
                    A few decisions shape everything else in {siteConfig.name}. Each one
                    keeps your data on your machine and under your control.
                </p>
                <ul>
                    <li>
                        <strong>Two-stream capture.</strong> System audio and the
                        microphone are recorded as separate streams. The microphone track
                        is always labelled You.
                    </li>
                    <li>
                        <strong>On-device transcription.</strong> The bundled backend is{" "}
                        <code>whisper.cpp</code> through the <code>whisper-rs</code>{" "}
                        bindings, Metal-accelerated on Apple Silicon. The cloud is an
                        opt-in fallback, never the default.
                    </li>
                    <li>
                        <strong>On-device diarization.</strong> Voices on the system-audio
                        track are clustered into Speaker 1, Speaker 2, and so on, with no
                        cloud involved.
                    </li>
                    <li>
                        <strong>One markdown note per meeting.</strong> Each meeting
                        becomes a single markdown file in your vault, with frontmatter for
                        attendees, duration, model, and source.
                    </li>
                    <li>
                        <strong>A local MCP server.</strong> <code>folio-mcp</code> gives
                        MCP-aware tools read-only access to your transcripts, tasks, and
                        memories over stdio. No cloud, no proxy.
                    </li>
                    <li>
                        <strong>Privacy Mode.</strong> A single switch physically blocks
                        every outbound HTTP call except <code>localhost</code>.
                    </li>
                </ul>
            </Prose>

            <DocH2 id="how-the-pieces-fit">How the pieces fit</DocH2>
            <Prose>
                <p>
                    {siteConfig.name} runs in your menu bar and watches for meetings. When
                    one starts it captures system audio and your microphone, transcribes
                    the audio on-device, separates the speakers, and writes a finished
                    markdown note into your vault. The vault is just a folder of files.
                    From there your notes are searchable on disk, readable in any editor,
                    and reachable by MCP-aware tools through the local server.
                </p>
            </Prose>

            <DocH2 id="start-here">Start here</DocH2>
            <Prose>
                <p>Pick the path that matches what you need next.</p>
            </Prose>
            <CardGrid>
                <LinkCard href="/docs/installation" title="Installation">
                    Install with Homebrew or a signed direct download, and grant the macOS
                    permissions {siteConfig.name} needs.
                </LinkCard>
                <LinkCard href="/docs/how-to-use" title="How to use">
                    Record a meeting, read the resulting note, and learn how speakers and
                    the You track appear.
                </LinkCard>
                <LinkCard href="/docs/architecture" title="Architecture">
                    The Rust core, the Tauri desktop binary, the three crates, and how
                    data flows from capture to disk.
                </LinkCard>
                <LinkCard href="/docs/connectors" title="Connectors">
                    Wire up the local <code>folio-mcp</code> server so Claude, Cursor, and
                    other MCP tools can read your notes.
                </LinkCard>
                <LinkCard href="/docs/privacy" title="Privacy">
                    What Folio does and does not send, Privacy Mode, encryption at rest,
                    and recording consent.
                </LinkCard>
                <LinkCard href="/docs/faq" title="FAQ">
                    Short answers to common questions about platforms, cost, cloud
                    fallback, and your data.
                </LinkCard>
            </CardGrid>
        </>
    );
}
