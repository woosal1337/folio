import * as React from "react";
import { ArrowLeft, FileAudio, Loader2, Sparkles } from "lucide-react";
import { Link, useLocation, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";

import { AudioPlayer } from "@/features/recording/audio-player";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { Separator } from "@/shared/ui/separator";
import { formatBytes, formatDuration } from "@/shared/lib/utils";
import { getRecording, readTranscript, transcribeRecording } from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";

import { AgentPanel } from "./agent-panel";
import { TranscriptEditor } from "./transcript-editor";

interface LocationState {
  recording?: RecordingSummary;
}

export default function Editor() {
  const navigate = useNavigate();
  const { label = "" } = useParams<{ label: string }>();
  const location = useLocation();
  const stateFromNav = (location.state as LocationState | null)?.recording;

  const [recording, setRecording] = React.useState<RecordingSummary | null>(
    stateFromNav ?? null
  );
  const [recordingLoading, setRecordingLoading] = React.useState(!stateFromNav);
  const [notFound, setNotFound] = React.useState(false);

  const [transcript, setTranscript] = React.useState<SessionTranscript | null>(null);
  const [transcriptLoading, setTranscriptLoading] = React.useState(false);
  const [transcriptError, setTranscriptError] = React.useState<string | null>(null);

  const transcribingDir = useRecording((s) => s.transcribingDir);
  const lastTranscriptPath = useRecording((s) => s.lastTranscriptPath);

  // Fetch the RecordingSummary when we don't already have it from
  // router state (deep link or hard reload).
  React.useEffect(() => {
    if (recording) return;
    if (!label) {
      setNotFound(true);
      return;
    }
    let cancelled = false;
    setRecordingLoading(true);
    getRecording(label)
      .then((r) => {
        if (cancelled) return;
        if (r) {
          setRecording(r);
        } else {
          setNotFound(true);
        }
      })
      .catch((e) => {
        if (cancelled) return;
        console.error("get_recording:", e);
        toast.error("Could not load recording", { description: String(e) });
        setNotFound(true);
      })
      .finally(() => {
        if (cancelled) return;
        setRecordingLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [label, recording]);

  const loadTranscript = React.useCallback(async (sessionDir: string) => {
    setTranscriptLoading(true);
    setTranscriptError(null);
    try {
      const t = await readTranscript(sessionDir);
      setTranscript(t);
    } catch (e) {
      setTranscriptError(String(e));
    } finally {
      setTranscriptLoading(false);
    }
  }, []);

  // Load (or reload) the transcript whenever the recording's transcript
  // status changes — including when a fresh transcription just landed.
  React.useEffect(() => {
    if (recording?.has_transcript) {
      loadTranscript(recording.session_dir);
    } else {
      setTranscript(null);
    }
  }, [recording, loadTranscript, lastTranscriptPath]);

  // Refresh the recording metadata once a transcription completes for
  // this session — flips has_transcript so the editor renders.
  React.useEffect(() => {
    if (!label || !lastTranscriptPath) return;
    getRecording(label)
      .then((r) => {
        if (r) setRecording(r);
      })
      .catch((e) => console.error("get_recording on transcript complete:", e));
  }, [label, lastTranscriptPath]);

  const handleTranscribe = async () => {
    if (!recording) return;
    try {
      await transcribeRecording(recording.session_dir);
    } catch (e) {
      console.error("transcribe_recording:", e);
      toast.error("Could not start transcription", { description: String(e) });
    }
  };

  // ---- Render guards ---------------------------------------------------

  if (notFound) {
    return (
      <CenteredPage>
        <h1 className="font-serif text-2xl font-medium">Recording not found</h1>
        <p className="max-w-md text-sm text-muted-foreground">
          The recording <span className="font-mono">{label}</span> does not exist in
          your configured recordings folder. It may have been deleted or renamed.
        </p>
        <Button onClick={() => navigate("/library")} className="gap-2">
          <ArrowLeft className="h-3.5 w-3.5" />
          Back to Library
        </Button>
      </CenteredPage>
    );
  }

  if (recordingLoading || !recording) {
    return (
      <CenteredPage>
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
        <p className="text-sm text-muted-foreground">Loading recording…</p>
      </CenteredPage>
    );
  }

  // ---- Main render -----------------------------------------------------

  const totalBytes =
    Number(recording.mic_bytes ?? 0n) + Number(recording.system_bytes ?? 0n);
  const micPath = recording.mic_bytes ? `${recording.session_dir}/mic.wav` : null;
  const systemPath = recording.system_bytes
    ? `${recording.session_dir}/system.wav`
    : null;
  const isCurrentlyTranscribing = transcribingDir === recording.session_dir;

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-10">
      <header data-drag="" className="select-none">
        <nav className="flex items-center gap-2 text-xs text-muted-foreground">
          <Link
            to="/library"
            className="inline-flex items-center gap-1 hover:text-foreground"
          >
            <ArrowLeft className="h-3 w-3" />
            Library
          </Link>
          <span>/</span>
          <span className="font-mono">{recording.label}</span>
        </nav>
        <div className="mt-3 flex flex-wrap items-baseline justify-between gap-3">
          <h1 className="font-serif text-3xl font-medium tracking-tight">
            {recording.label}
          </h1>
          <div className="flex items-center gap-2 font-mono text-xs text-muted-foreground">
            <span>{formatDuration(Number(recording.duration_seconds))}</span>
            <span>·</span>
            <span>{formatBytes(totalBytes)}</span>
            {recording.has_transcript && (
              <Badge variant="accent" className="gap-1.5 text-2xs">
                <Sparkles className="h-3 w-3" />
                transcribed
              </Badge>
            )}
          </div>
        </div>
      </header>

      <Card>
        <CardContent className="flex flex-col gap-4 py-5">
          {micPath ? (
            <AudioPlayer filePath={micPath} label="Mic" />
          ) : (
            <p className="text-xs text-muted-foreground">No mic track.</p>
          )}
          {systemPath && <Separator />}
          {systemPath && <AudioPlayer filePath={systemPath} label="System" />}
        </CardContent>
      </Card>

      <Card>
        <CardContent className="py-5">
          {isCurrentlyTranscribing ? (
            <div
              className="flex items-center gap-2 text-sm text-muted-foreground"
              role="status"
              aria-live="polite"
            >
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Sending audio to OpenAI Whisper…</span>
            </div>
          ) : !recording.has_transcript ? (
            <div className="flex flex-col items-start gap-3">
              <div className="flex items-center gap-2">
                <FileAudio className="h-4 w-4 text-muted-foreground" />
                <p className="text-sm">No transcript yet for this recording.</p>
              </div>
              <Button onClick={handleTranscribe} className="gap-2">
                <Sparkles className="h-3.5 w-3.5" />
                Transcribe now
              </Button>
              <p className="text-xs text-muted-foreground">
                Uses the OpenAI Whisper API. Configure your key in Settings →
                Transcription.
              </p>
            </div>
          ) : transcriptLoading ? (
            <div
              className="flex items-center gap-2 text-sm text-muted-foreground"
              role="status"
              aria-live="polite"
            >
              <Loader2 className="h-4 w-4 animate-spin" />
              <span>Loading transcript…</span>
            </div>
          ) : transcriptError ? (
            <p className="text-sm text-destructive">{transcriptError}</p>
          ) : transcript ? (
            <TranscriptEditor
              sessionDir={recording.session_dir}
              initial={transcript}
              onSaved={(next) => setTranscript(next)}
            />
          ) : null}
        </CardContent>
      </Card>

      {recording.has_transcript && transcript ? (
        <Card>
          <CardContent className="py-5">
            <AgentPanel sessionDir={recording.session_dir} />
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}

function CenteredPage({ children }: { children: React.ReactNode }) {
  return (
    <div className="mx-auto flex h-full w-full max-w-md flex-col items-center justify-center gap-4 px-8 py-16 text-center">
      {children}
    </div>
  );
}
