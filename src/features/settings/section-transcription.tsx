import * as React from "react";
import { useNavigate } from "react-router-dom";
import { Captions, CircleUserRound, KeyRound, Server, Zap } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import { cn } from "@/shared/lib/utils";
import { humanizeError } from "@/shared/lib/errors";
import { listProviders, setProviderKey, whisperModelStatus } from "@/shared/lib/ipc";
import { useRemoteAccountStore } from "@/shared/stores/remote-account-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";
import type { ProviderStatus } from "@/shared/types/ProviderStatus";
import type { Settings } from "@/shared/types/Settings";
import type { WhisperModelStatus } from "@/shared/types/WhisperModelStatus";

import { LocalWhisperSection } from "./local-whisper-section";
import { SpeakerDiarizationSection } from "./speaker-diarization-section";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

const PROVIDERS: { id: string; label: string; desc: string }[] = [
  {
    id: "openai",
    label: "OpenAI Whisper API",
    desc: "Uploaded to OpenAI · ~$0.006/min · multilingual",
  },
  {
    id: "local_whisper",
    label: "Local Whisper",
    desc: "Runs on this Mac via whisper.cpp · no audio leaves your machine",
  },
  {
    id: "remote_server",
    label: "Remote server",
    desc: "Uploads to your GPU server · frees this Mac · syncs the transcript back",
  },
];

const LANGUAGES: { value: string; label: string }[] = [
  { value: "auto", label: "Auto-detect" },
  { value: "en", label: "English" },
  { value: "tr", label: "Turkish" },
  { value: "az", label: "Azerbaijani" },
  { value: "ru", label: "Russian" },
  { value: "de", label: "German" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
  { value: "ar", label: "Arabic" },
  { value: "ja", label: "Japanese" },
  { value: "zh", label: "Chinese" },
];

export function SectionTranscription({ settings, onChange }: Props) {
  const [whisperStatus, setWhisperStatus] = React.useState<WhisperModelStatus | null>(
    null
  );
  const [whisperStatusLoading, setWhisperStatusLoading] = React.useState(true);

  const refreshWhisperStatus = React.useCallback(async () => {
    setWhisperStatusLoading(true);
    try {
      setWhisperStatus(await whisperModelStatus());
    } catch (e) {
      console.error("whisper_model_status:", e);
    } finally {
      setWhisperStatusLoading(false);
    }
  }, []);

  React.useEffect(() => {
    void refreshWhisperStatus();
  }, [refreshWhisperStatus, settings.local_whisper_model]);

  const whisperReady =
    (whisperStatus?.present ?? false) &&
    whisperStatus?.id === settings.local_whisper_model;
  const isLocalProvider = settings.transcriber === "local_whisper";
  const liveTranscriptReady = whisperReady && isLocalProvider;

  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Transcription</h2>

      <div className="flex items-start justify-between gap-6 rounded-lg border border-border bg-card p-4">
        <div className="space-y-1">
          <Label
            htmlFor="auto-transcribe-toggle"
            className="flex items-center gap-2 text-sm font-medium"
          >
            <Zap className="h-4 w-4 text-muted-foreground" />
            Auto-transcribe after recording
            <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
              Recommended
            </span>
          </Label>
          <p className="max-w-md text-xs text-muted-foreground">
            Start transcribing as soon as you stop a recording, using the provider
            selected below. Skipped silently if the OpenAI Whisper API is selected
            without a key. Turn this off if you prefer to transcribe manually from the
            Library.
          </p>
        </div>
        <Switch
          id="auto-transcribe-toggle"
          checked={settings.auto_transcribe_enabled}
          onCheckedChange={(checked) => onChange("auto_transcribe_enabled", checked)}
          className="mt-1"
        />
      </div>

      <div className="flex items-start justify-between gap-6 rounded-lg border border-border bg-card p-4">
        <div className="space-y-1">
          <Label
            htmlFor="auto-vad-toggle"
            className="flex items-center gap-2 text-sm font-medium"
          >
            <Zap className="h-4 w-4 text-muted-foreground" />
            Strip silence before transcription
            <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
              Recommended
            </span>
          </Label>
          <p className="max-w-md text-xs text-muted-foreground">
            Runs a fast voice-activity-detection pass on the mic and system tracks
            before sending them to the transcriber. Removes silent stretches so the
            model never gets a chance to hallucinate over them, and cuts cloud-Whisper
            upload size on meetings with long listening periods.
          </p>
        </div>
        <Switch
          id="auto-vad-toggle"
          checked={settings.auto_vad_enabled}
          onCheckedChange={(checked) => onChange("auto_vad_enabled", checked)}
          className="mt-1"
        />
      </div>

      <div className="flex items-start justify-between gap-6 rounded-lg border border-border bg-card p-4">
        <div className="space-y-1">
          <Label
            htmlFor="live-transcript-toggle"
            className="flex items-center gap-2 text-sm font-medium"
          >
            <Captions className="h-4 w-4 text-muted-foreground" />
            Live transcription
            <span className="rounded-full border border-primary/40 px-1.5 py-px text-[10px] font-medium uppercase tracking-wider text-primary">
              Beta
            </span>
          </Label>
          <p className="max-w-md text-xs text-muted-foreground">
            Streams a live caption into the record dock while you&apos;re recording,
            using local Whisper over a rolling window. Still experimental — when off,
            Folio transcribes once automatically as soon as the recording stops.
            Requires a downloaded local Whisper model.
          </p>
          {!isLocalProvider ? (
            <p className="text-2xs font-medium text-amber-600 dark:text-amber-400">
              Switch the provider to Local Whisper to use live transcription.
            </p>
          ) : !whisperStatusLoading && !whisperReady ? (
            <p className="text-2xs font-medium text-amber-600 dark:text-amber-400">
              Download the selected local Whisper model below to turn this on.
            </p>
          ) : null}
        </div>
        <Switch
          id="live-transcript-toggle"
          checked={settings.live_transcript_enabled && liveTranscriptReady}
          disabled={whisperStatusLoading || !liveTranscriptReady}
          onCheckedChange={(checked) => onChange("live_transcript_enabled", checked)}
          className="mt-1"
        />
      </div>

      <section className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Provider
        </Label>
        <div className="grid gap-1.5">
          {PROVIDERS.map((p) => {
            const selected = settings.transcriber === p.id;
            return (
              <button
                type="button"
                key={p.id}
                onClick={() => onChange("transcriber", p.id)}
                aria-pressed={selected}
                className={cn(
                  "flex items-center justify-between gap-3 rounded-md border px-3 py-2 text-left transition-colors",
                  selected
                    ? "border-primary bg-accent"
                    : "border-border bg-card hover:bg-secondary"
                )}
              >
                <div className="flex min-w-0 flex-col gap-0.5">
                  <span className="text-sm font-medium">{p.label}</span>
                  <span className="truncate text-xs text-muted-foreground">
                    {p.desc}
                  </span>
                </div>
                {selected && (
                  <Badge variant="accent" className="shrink-0 text-2xs">
                    Selected
                  </Badge>
                )}
              </button>
            );
          })}
        </div>
      </section>

      {settings.transcriber === "openai" && <OpenAiKeySection />}

      {settings.transcriber === "remote_server" && (
        <RemoteServerSection settings={settings} onChange={onChange} />
      )}

      {settings.transcriber === "local_whisper" && (
        <LocalWhisperSection
          settings={settings}
          onChange={onChange}
          status={whisperStatus}
          statusLoading={whisperStatusLoading}
          refreshStatus={refreshWhisperStatus}
          onStatusChange={setWhisperStatus}
        />
      )}

      <SpeakerDiarizationSection settings={settings} onChange={onChange} />

      <section className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Language
        </Label>
        <select
          value={settings.transcription_language}
          onChange={(e) => onChange("transcription_language", e.target.value)}
          className="h-9 w-full rounded-md border border-input bg-card px-3 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          {LANGUAGES.map((l) => (
            <option key={l.value} value={l.value}>
              {l.label}
            </option>
          ))}
        </select>
        <p className="text-xs text-muted-foreground">
          Set a language if you record predominantly in one. Auto detects per segment.
        </p>
      </section>
    </div>
  );
}

function OpenAiKeySection() {
  const [status, setStatus] = React.useState<ProviderStatus | null>(null);
  const [draft, setDraft] = React.useState("");
  const [saving, setSaving] = React.useState(false);

  const refresh = React.useCallback(async () => {
    try {
      const list = await listProviders();
      setStatus(list.find((p) => p.id === "openai") ?? null);
    } catch (e) {
      console.error("listProviders:", e);
    }
  }, []);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const list = await listProviders();
        if (!cancelled) setStatus(list.find((p) => p.id === "openai") ?? null);
      } catch (e) {
        if (!cancelled) console.error("listProviders:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const onSave = async () => {
    const next = draft.trim();
    if (next.length === 0) {
      toast.error("Enter a key first");
      return;
    }
    setSaving(true);
    try {
      await setProviderKey("openai", next);
      setDraft("");
      toast.success("OpenAI key saved to Keychain");
      await refresh();
    } catch (e) {
      console.error("set_provider_key:", e);
      toast.error("Could not save key", { description: humanizeError(e) });
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="space-y-3">
      <Label
        htmlFor="openai-key"
        className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground"
      >
        <KeyRound className="h-3.5 w-3.5" />
        OpenAI API key
        {status?.configured ? (
          <Badge variant="accent" className="text-2xs">
            Stored · {status.redacted_suffix ?? "key set"}
          </Badge>
        ) : null}
      </Label>
      <div className="flex gap-2">
        <Input
          id="openai-key"
          type="password"
          placeholder={status?.configured ? "•••• replace key" : "sk-..."}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          className="font-mono"
          autoComplete="off"
        />
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onSave}
          disabled={saving || draft.trim().length === 0}
        >
          {saving ? "Saving…" : status?.configured ? "Replace" : "Save"}
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">
        Stored in the macOS Keychain. Never persisted to disk; never logged. Sent only
        to api.openai.com when transcribing.
      </p>
    </section>
  );
}

function RemoteServerSection({
  settings,
  onChange,
}: {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}) {
  const navigate = useNavigate();
  const closeSettings = useSettingsUiStore((s) => s.close);
  const account = useRemoteAccountStore((s) => s.account);
  const refreshAccount = useRemoteAccountStore((s) => s.refresh);

  React.useEffect(() => {
    void refreshAccount();
  }, [refreshAccount]);

  const endpoint = settings.remote_endpoint.trim();

  return (
    <section className="space-y-4">
      <div className="space-y-3 rounded-lg border border-border bg-card p-4">
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0 space-y-1">
            <p className="flex items-center gap-2 text-sm font-medium">
              <Server className="h-4 w-4 text-muted-foreground" />
              Folio Server
            </p>
            <p className="truncate font-mono text-xs text-muted-foreground">
              {endpoint || "No endpoint configured"}
            </p>
          </div>
          {account?.signed_in ? (
            <Badge variant="accent" className="shrink-0 text-2xs">
              {account.email ?? "Signed in"}
            </Badge>
          ) : (
            <Badge variant="outline" className="shrink-0 text-2xs">
              Not signed in
            </Badge>
          )}
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="gap-1.5"
          onClick={() => {
            closeSettings();
            navigate("/account");
          }}
        >
          <CircleUserRound className="h-3.5 w-3.5" />
          Manage in Account
        </Button>
        <p className="text-xs text-muted-foreground">
          Endpoint, connection test, and sign-in live in the Account tab.
        </p>
      </div>

      <div className="flex items-center justify-between">
        <div className="space-y-0.5">
          <Label htmlFor="remote-auto-upload" className="text-sm">
            Auto-upload recordings
          </Label>
          <p className="text-xs text-muted-foreground">
            When a recording stops, upload it to the server and sync the transcript
            back.
          </p>
        </div>
        <Switch
          id="remote-auto-upload"
          checked={settings.remote_auto_upload}
          onCheckedChange={(checked) => onChange("remote_auto_upload", checked)}
        />
      </div>

      <p className="text-xs text-muted-foreground">
        Audio is uploaded to your server and processed there. Tokens are stored in the
        macOS Keychain. Disabled entirely in Privacy Mode.
      </p>
    </section>
  );
}
