import type { Metadata } from "next";
import { siteConfig } from "@/lib/site-config";
import { CodeBlock } from "@/components/site/code-block";
import {
  DocHeader,
  DocH2,
  Prose,
  Callout,
  FieldList,
  Field,
} from "@/components/docs/doc-primitives";

export const metadata: Metadata = {
  title: "Architecture",
  description:
    "How Folio is built, from the Rust core through the Tauri shell to the data flow that moves a meeting from microphone to markdown.",
};

const repositoryLayout = `folio/
  Cargo.toml            workspace root
  rust-toolchain.toml   Rust 1.88, both Apple targets
  crates/
    folio-core/         audio capture, storage, transcription, diarization
    folio-cli/          CLI test harness
    folio-mcp/          local MCP stdio server (notes, tasks, memories)
  src-tauri/            Tauri 2 desktop binary
  src/                  React 18 + TypeScript + Tailwind frontend
  Casks/                Homebrew cask (the repo doubles as its own tap)
  docs/                 repo-local documentation`;

const dataFlow = `React (src/)
  features/* + Zustand stores + shared/lib/ipc.ts
        |  invoke: JSON over Tauri IPC
        v
src-tauri/
  commands/* + app/state.rs + folio-core re-exports
        |  direct function calls
        v
folio-core
  audio:: + llm:: + memory:: + storage:: + transcription::
        |  OS APIs + OpenAI + whisper.cpp + SQLite
        v
Disk + Hardware + Network`;

export default function ArchitecturePage() {
  return (
    <>
      <DocHeader
        eyebrow="Going deeper"
        title="Architecture"
        description={`How ${siteConfig.name} is built, from the Rust core through the Tauri shell to the data flow that moves a meeting from microphone to markdown.`}
      />

      <DocH2 id="the-stack">The stack</DocH2>
      <Prose>
        <p>
          {siteConfig.name} is a Rust program with a desktop face. About 70
          percent of the code is a Rust core that owns audio capture, storage,
          and transcription. That core is wrapped by a Tauri 2 desktop binary,
          which gives it a native window, OS permissions, and a menu-bar
          presence without shipping a full browser engine.
        </p>
        <p>
          The interface is a React 18 frontend written in TypeScript and styled
          with Tailwind. The two halves talk over Tauri IPC. The types that
          cross that boundary are not written twice. They are defined once in
          Rust and generated to TypeScript with <code>ts-rs</code>, so the
          frontend always sees the same shapes the core returns.
        </p>
      </Prose>

      <DocH2 id="repository-layout">Repository layout</DocH2>
      <Prose>
        <p>
          The repository is a single Cargo workspace plus the frontend and the
          packaging it ships with. The Rust toolchain is pinned, the crates are
          split by responsibility, and the Homebrew cask lives in the repository
          itself, so the project doubles as its own tap.
        </p>
      </Prose>
      <CodeBlock label="folio/" code={repositoryLayout} />

      <DocH2 id="the-rust-core">The Rust core</DocH2>
      <Prose>
        <p>
          The workspace holds three crates. Each has one job and a clear edge
          against the others.
        </p>
      </Prose>
      <FieldList>
        <Field name="folio-core" type="crate">
          The framework-agnostic core. It owns audio capture, storage,
          transcription, diarization, the agent, and the memory and task stores.
          It is embeddable by the Tauri app, by the CLI, or by a future Swift
          app via UniFFI.
        </Field>
        <Field name="folio-cli" type="crate">
          A CLI test harness. It exercises the core from the terminal without
          the desktop shell, which makes audio devices and recording easy to
          probe in isolation.
        </Field>
        <Field name="folio-mcp" type="crate">
          The local MCP stdio server. It exposes notes, tasks, and memories to
          MCP-aware tools with read-only access and no network hop.
        </Field>
      </FieldList>
      <Prose>
        <p>
          A few rules hold across the core. <code>FolioError</code> is the
          single public error type, so callers handle one shape instead of a
          dozen. Logging goes through the <code>tracing</code> crate, never{" "}
          <code>println</code>, and audio callbacks are alloc-free hot paths
          that never log inside the callback body. macOS-specific code is gated
          behind a <code>cfg(target_os = &quot;macos&quot;)</code> attribute,
          with stubs for other targets so the whole workspace still builds
          everywhere.
        </p>
      </Prose>

      <DocH2 id="capture-pipeline">Capture pipeline</DocH2>
      <Prose>
        <p>
          When a meeting starts, {siteConfig.name} records two independent
          streams. <code>cpal</code> captures the microphone. ScreenCaptureKit
          captures system audio, which is everyone else on the call. Keeping the
          streams separate is what makes the rest of the pipeline honest. Your
          voice and the room never get mixed into one undecodable track.
        </p>
        <p>
          The two streams rarely share a sample rate, so <code>rubato</code>{" "}
          resamples them to a common rate, and <code>hound</code> writes the
          result to WAV files on disk. The microphone track is always labelled{" "}
          <code>You</code>, which the transcription and diarization steps then
          rely on.
        </p>
      </Prose>

      <DocH2 id="transcription-and-diarization">
        Transcription and diarization
      </DocH2>
      <Prose>
        <p>
          Transcription runs locally by default. The bundled backend is
          whisper.cpp through the <code>whisper-rs</code> bindings,
          Metal-accelerated on Apple Silicon. This is the primary path and it
          needs no network once the weights are present. The OpenAI Whisper API
          is an opt-in fallback for faster cloud transcription on long meetings.
          It needs an OpenAI key and it is never the default.
        </p>
        <p>
          Diarization runs on-device against the system-audio track. It uses a
          pyannote-segmentation-3.0 model plus a WeSpeaker speaker-embedding
          model, both run through sherpa-onnx, then clusters the voices into
          Speaker 1, Speaker 2, Speaker 3 and so on. The microphone is always{" "}
          <code>You</code>, and no cloud is involved in diarization at all.
        </p>
      </Prose>

      <DocH2 id="the-ipc-contract">The IPC contract</DocH2>
      <Prose>
        <p>
          Every Tauri command is the contract between the core and the frontend.
          The argument and return types of those commands are defined in{" "}
          <code>folio-core</code> and generated to TypeScript by{" "}
          <code>ts-rs</code>. There is no second source of truth to keep in
          sync. Running <code>cargo test</code> regenerates the bindings, and CI
          catches any drift between what Rust declares and what TypeScript
          expects, so a changed signature cannot silently reach the UI.
        </p>
      </Prose>
      <Callout variant="tip" title="Two-phase writes">
        <p>
          File-backed stores write in two phases. The canonical on-disk file
          lands first. That is the <code>.md</code> for notes and the{" "}
          <code>.json</code> for tasks. The derived index is written second. The
          index is always rebuildable from the files, so the files are the
          source of truth and the index is just a fast read path you can throw
          away and regenerate.
        </p>
      </Callout>

      <DocH2 id="data-flow">Data flow</DocH2>
      <Prose>
        <p>
          Top to bottom, a request starts in React, crosses the IPC boundary as
          JSON, lands in the Tauri commands layer, and calls into{" "}
          <code>folio-core</code>, which is the only layer that talks to the OS,
          to OpenAI, to whisper.cpp, and to SQLite.
        </p>
      </Prose>
      <CodeBlock label="data flow" code={dataFlow} />
      <Prose>
        <p>
          This page is the working summary. For the full account, including the
          module boundaries inside <code>folio-core</code> and the reasoning
          behind each layer, read the{" "}
          <a
            href={siteConfig.links.architecture}
            target="_blank"
            rel="noreferrer"
          >
            architecture document
          </a>{" "}
          in the repository.
        </p>
      </Prose>
    </>
  );
}
