import type { Metadata } from "next";
import Link from "next/link";
import { siteConfig } from "@/lib/site-config";
import { CodeBlock, CommandLine } from "@/components/site/code-block";
import {
  DocHeader,
  DocH2,
  Prose,
  Callout,
  Steps,
  Step,
} from "@/components/docs/doc-primitives";

export const metadata: Metadata = {
  title: "Installation",
  description: `Install ${siteConfig.name} on ${siteConfig.platform} with Homebrew or the notarized DMG, then grant the permissions it needs to capture audio.`,
};

const homebrewCommands = `${siteConfig.install.tapCommand}\n${siteConfig.install.installCommand}`;

const sourceSetupCommands = `git clone ${siteConfig.links.github}
cd folio
bun install
pre-commit install
pre-commit install --hook-type commit-msg
pre-commit install --hook-type pre-push`;

export default function InstallationPage() {
  return (
    <>
      <DocHeader
        eyebrow="Getting started"
        title="Install Folio"
        description="Folio is a menu-bar app for macOS. With Homebrew you can be ready in about a minute."
      />

      <DocH2 id="requirements">Requirements</DocH2>
      <Prose>
        <p>
          Folio runs on <strong>{siteConfig.platform}</strong>. Apple Silicon is
          the performance target, and Intel Macs are supported by building from
          source. Local transcription is Metal-accelerated on Apple Silicon.
        </p>
        <ul>
          <li>
            <strong>macOS 13 Ventura or later</strong>, on Apple Silicon or
            Intel.
          </li>
          <li>
            A <strong>one-time model-weights download</strong> on first use. The
            first local transcription or diarization fetches a few hundred
            megabytes of weights, then Folio works offline.
          </li>
        </ul>
      </Prose>

      <DocH2 id="homebrew">Homebrew</DocH2>
      <Prose>
        <p>
          Homebrew is the recommended way to install Folio. Tap the cask, then
          install it. The two commands below run in order.
        </p>
      </Prose>
      <CodeBlock code={homebrewCommands} label="Recommended" />
      <Prose>
        <p>
          Once installed, <code>{siteConfig.install.upgradeCommand}</code> tracks
          new releases and keeps your copy current.
        </p>
      </Prose>

      <DocH2 id="direct-download">Direct download</DocH2>
      <Prose>
        <p>
          If you prefer a direct download, grab the latest Apple Silicon{" "}
          <code>.dmg</code> from the{" "}
          <a href={siteConfig.links.releases} target="_blank" rel="noreferrer">
            Releases page
          </a>
          . The file is named <code>Folio_&lt;version&gt;_aarch64.dmg</code>.
          Open it and drag Folio to your Applications folder.
        </p>
        <p>
          Releases are code-signed with a Developer ID and notarized by Apple, so
          they open without a Gatekeeper prompt. Intel builds are not published
          yet. On Intel, build from source.
        </p>
      </Prose>

      <DocH2 id="build-from-source">Build from source</DocH2>
      <Steps>
        <Step n={1} title="Install the prerequisites">
          <Prose>
            <p>You need three toolchains in place before building.</p>
            <ul>
              <li>
                <strong>Rust 1.88</strong> via <code>rustup</code>, pinned in{" "}
                <code>rust-toolchain.toml</code>.
              </li>
              <li>
                <strong>Bun 1.3+</strong>, the only JavaScript package manager
                and runtime this repository uses.
              </li>
              <li>
                <strong>Xcode command-line tools</strong>, installed with{" "}
                <code>xcode-select --install</code>.
              </li>
            </ul>
          </Prose>
        </Step>
        <Step n={2} title="Clone and set up the repository">
          <Prose>
            <p>
              Clone the repo, install dependencies with Bun, and register the
              pre-commit hooks.
            </p>
          </Prose>
          <CodeBlock code={sourceSetupCommands} label="Setup" />
        </Step>
        <Step n={3} title="Run the desktop app">
          <Prose>
            <p>
              Start the app in development. The first launch compiles the Rust
              workspace in about 30 seconds on a warm cache.
            </p>
          </Prose>
          <CommandLine command="bun tauri dev" />
        </Step>
      </Steps>

      <DocH2 id="grant-permissions">Grant permissions</DocH2>
      <Prose>
        <p>
          On first run, macOS prompts for the permissions Folio needs to capture
          a meeting. Folio watches your calendar and audio devices through
          EventKit. No bot joins the call.
        </p>
      </Prose>
      <Steps>
        <Step n={1} title="Allow microphone access">
          <Prose>
            <p>
              macOS asks for microphone permission so Folio can record your
              voice. Your microphone track is always labelled <code>You</code>.
            </p>
          </Prose>
        </Step>
        <Step n={2} title="Allow screen recording">
          <Prose>
            <p>
              macOS asks for screen recording permission. Folio uses it to
              capture system audio, the other side of the conversation, as a
              separate stream from your microphone.
            </p>
          </Prose>
        </Step>
      </Steps>
      <Callout variant="warning" title="Screen recording is required">
        <p>
          System audio capture on macOS goes through ScreenCaptureKit, which is
          gated behind the screen recording permission. Without it, Folio can
          record your microphone but not the other participants. Grant screen
          recording so both streams are captured.
        </p>
      </Callout>

      <DocH2 id="update-and-uninstall">Update and uninstall</DocH2>
      <Prose>
        <p>To update Folio to the latest release, run the upgrade command.</p>
      </Prose>
      <CommandLine command={siteConfig.install.upgradeCommand} />
      <Prose>
        <p>
          To remove the app, run <code>brew uninstall --cask folio</code>.
          Uninstalling removes the application only. Your markdown vault is left
          untouched, since notes are plain files on disk that you own. You can
          read, search, or back them up with any tool, with or without Folio
          installed.
        </p>
        <p>
          For the full source and signed releases, see the{" "}
          <Link href="/docs/how-to-use">how to use</Link> guide, or the project
          on{" "}
          <a href={siteConfig.links.github} target="_blank" rel="noreferrer">
            GitHub
          </a>
          .
        </p>
      </Prose>
    </>
  );
}
