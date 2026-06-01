import * as React from "react";
import {
  ArrowLeft,
  Check,
  ChevronRight,
  Copy,
  FileText,
  Loader2,
  MessageCircleQuestion,
  Mic,
  MoreHorizontal,
  Pause,
  Play,
  RefreshCw,
  Share,
  Sparkles,
  Square,
  Trash2,
  User as UserIcon,
} from "lucide-react";
import { Link, useLocation, useNavigate, useParams } from "react-router-dom";
import { toast } from "sonner";

import { AudioPlayer } from "@/features/recording/audio-player";
import { EnhancedNotesBody } from "@/features/editor/enhanced-notes";
import { MarkdownNotesEditor } from "@/features/recording/markdown-notes-editor";
import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Separator } from "@/shared/ui/separator";
import { copyToClipboard } from "@/shared/lib/share";
import { formatBytes, formatDuration } from "@/shared/lib/utils";
import {
  clearRecordingArtifacts,
  deleteRecording,
  exportNoteMarkdown,
  getEnhancedNotesAccepted,
  getRecording,
  listAgentRuns,
  setEnhancedNotesAccepted,
  onLiveTranscript,
  readTranscript,
  renameNote,
  revealInFinder,
  runAgent,
  sharePaths,
  transcribeRecording,
} from "@/shared/lib/ipc";
import { useRecording } from "@/shared/stores/recording-store";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { useJobsStore } from "@/shared/stores/jobs-store";
import { useTranscriberCopy } from "@/shared/hooks/use-transcriber-copy";
import type { AgentRun } from "@/shared/types/AgentRun";
import type { RecordingSummary } from "@/shared/types/RecordingSummary";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";

import { FolderChip } from "./folder-chip";
import { confirmDelete } from "@/shared/stores/confirm-delete-store";
import { serialiseAsPlainText } from "@/shared/lib/note-export";
import { ParticipantCards } from "./participant-cards";
import { TranscriptEditor } from "./transcript-editor";
import { FollowupEmailButton } from "@/features/recording/followup-email-button";
import { NoteChat } from "@/features/recording/note-chat";

interface LocationState {
  recording?: RecordingSummary;
}

export default function Editor() {
  const navigate = useNavigate();
  const { label = "" } = useParams<{ label: string }>();
  const location = useLocation();
  const navState = location.state as LocationState | null;
  const stateFromNav = navState?.recording;
  const [reTranscribing, setReTranscribing] = React.useState(false);
  const [regenerating, setRegenerating] = React.useState(false);
  const [chatOpen, setChatOpen] = React.useState(false);
  const [transcriptOpen, setTranscriptOpen] = React.useState(false);
  const transcriber = useTranscriberCopy();

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
  // Recording-store state for the in-note record dock (GET-155).
  const recState = useRecording();

  // Live-transcript preview (GET-160): the latest rolling-window caption
  // for THIS note while it's the active capture. Cleared when capture
  // stops; the final on-stop transcript is the source of truth. Gated
  // behind the Beta opt-in — when off, the backend never emits previews,
  // so the dock should not advertise a live caption either.
  const liveTranscriptEnabled = useSettingsStore(
    (s) => s.settings?.live_transcript_enabled ?? false
  );
  const [livePreview, setLivePreview] = React.useState("");
  const liveSessionDir = recState.liveSessionDir;
  const isCapturingThis =
    (recState.recording || recState.paused) &&
    liveSessionDir === recording?.session_dir;
  React.useEffect(() => {
    if (!isCapturingThis) {
      setLivePreview("");
      return;
    }
    let unlisten: (() => void) | undefined;
    void onLiveTranscript((p) => {
      if (p.session_dir === recording?.session_dir) setLivePreview(p.text);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [isCapturingThis, recording?.session_dir]);

  // Fetch the RecordingSummary when not provided via router state.
  React.useEffect(() => {
    if (recording) return;
    if (!label) {
      setNotFound(true);
      return;
    }
    let cancelled = false;
    setRecordingLoading(true);
    (async () => {
      try {
        const r = await getRecording(label);
        if (cancelled) return;
        if (r) setRecording(r);
        else setNotFound(true);
      } catch (e) {
        if (cancelled) return;
        console.error("get_recording:", e);
        toast.error("Could not load recording", { description: String(e) });
        setNotFound(true);
      } finally {
        if (!cancelled) setRecordingLoading(false);
      }
    })();
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

  React.useEffect(() => {
    if (recording?.has_transcript) {
      loadTranscript(recording.session_dir);
    } else {
      setTranscript(null);
    }
  }, [recording, loadTranscript, lastTranscriptPath]);

  // Refresh the recording metadata once a transcription completes.
  React.useEffect(() => {
    if (!label || !lastTranscriptPath) return;
    let cancelled = false;
    (async () => {
      try {
        const r = await getRecording(label);
        if (!cancelled && r) setRecording(r);
      } catch (e) {
        if (!cancelled) console.error("get_recording on transcript complete:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [label, lastTranscriptPath]);

  const [agentRuns, setAgentRuns] = React.useState<AgentRun[]>([]);
  const refreshRuns = React.useCallback(async () => {
    if (!recording?.session_dir || !recording.has_transcript) {
      setAgentRuns([]);
      return;
    }
    try {
      setAgentRuns(await listAgentRuns(recording.session_dir));
    } catch (e) {
      console.error("list_agent_runs:", e);
    }
  }, [recording?.session_dir, recording?.has_transcript]);

  React.useEffect(() => {
    void refreshRuns();
  }, [refreshRuns, lastTranscriptPath]);

  // Live-refresh as background work for THIS note finishes. The auto-fire
  // chain (transcribe → summarize → tasks/memories) runs after the page is
  // already open, and each step pushes/pops a job in the jobs store. When a
  // job for this session disappears (completed), re-fetch the agent runs so
  // the enhanced notes appear the moment they're ready — no need to leave
  // the page and come back.
  const jobs = useJobsStore((s) => s.jobs);
  const prevJobIds = React.useRef<Set<string>>(new Set());
  React.useEffect(() => {
    const dir = recording?.session_dir;
    if (!dir) return;
    const active = new Set(
      Object.values(jobs)
        .filter((j) => j.sessionDir === dir)
        .map((j) => j.id)
    );
    let completed = false;
    prevJobIds.current.forEach((id) => {
      if (!active.has(id)) completed = true;
    });
    prevJobIds.current = active;
    if (completed) void refreshRuns();
  }, [jobs, recording?.session_dir, refreshRuns]);

  const summaryRun = agentRuns.find((r) => r.agent_id === "summarize") ?? null;

  // GET-191: enhanced notes are AI-generated, so render them muted until the
  // user "keeps" (owns) them — the Granola gray→black provenance cue.
  // Acceptance is keyed to the run's finished_at, so a Regenerate (new run)
  // reverts to muted until kept again.
  const [acceptedMarker, setAcceptedMarker] = React.useState<string | null>(null);
  React.useEffect(() => {
    const dir = recording?.session_dir;
    if (!dir) {
      setAcceptedMarker(null);
      return;
    }
    let cancelled = false;
    void getEnhancedNotesAccepted(dir)
      .then((m) => {
        if (!cancelled) setAcceptedMarker(m);
      })
      .catch((e) => console.error("get_enhanced_notes_accepted:", e));
    return () => {
      cancelled = true;
    };
  }, [recording?.session_dir, summaryRun?.finished_at]);

  const enhancedNotesKept =
    summaryRun !== null && acceptedMarker === summaryRun.finished_at;

  const keepEnhancedNotes = React.useCallback(async () => {
    const dir = recording?.session_dir;
    if (!dir || !summaryRun) return;
    try {
      await setEnhancedNotesAccepted(dir, summaryRun.finished_at);
      setAcceptedMarker(summaryRun.finished_at);
    } catch (e) {
      console.error("set_enhanced_notes_accepted:", e);
      toast.error("Could not keep notes", { description: String(e) });
    }
  }, [recording?.session_dir, summaryRun]);

  // Is the summary being generated right now (auto-fire after transcribe,
  // or the manual Regenerate)? Used to show a loading state in place of the
  // empty "no enhanced notes yet" prompt, and to keep the note locked until
  // the pipeline settles.
  const summarizing = React.useMemo(
    () =>
      !!recording?.session_dir &&
      Object.values(jobs).some(
        (j) =>
          j.sessionDir === recording.session_dir && j.id.startsWith("agent:summarize:")
      ),
    [jobs, recording?.session_dir]
  );

  const handleTranscribe = async () => {
    if (!recording) return;
    try {
      await transcribeRecording(recording.session_dir);
    } catch (e) {
      console.error("transcribe_recording:", e);
      toast.error("Could not start transcription", { description: String(e) });
    }
  };

  const handleRegenerate = async () => {
    if (!recording) return;
    setRegenerating(true);
    try {
      await runAgent(recording.session_dir, "summarize");
      await refreshRuns();
      toast.success("Notes regenerated");
    } catch (e) {
      console.error("regenerate notes:", e);
      toast.error("Could not regenerate notes", { description: String(e) });
    } finally {
      setRegenerating(false);
    }
  };

  const handleCopy = async () => {
    if (!recording) return;
    try {
      await copyToClipboard(
        serialiseAsPlainText({
          recording,
          summary: summaryRun,
          tasks: agentRuns.find((r) => r.agent_id === "extract-tasks") ?? null,
          memories: agentRuns.find((r) => r.agent_id === "extract-memories") ?? null,
        })
      );
      toast.success("Notes copied to clipboard");
    } catch (e) {
      toast.error("Could not copy", { description: String(e) });
    }
  };

  // GET-166: export the note as a self-contained Markdown file and hand
  // it to the OS share sheet (AirDrop / Mail / Messages / Notes). If the
  // share sheet isn't available, reveal the file in Finder instead. All
  // local — no cloud egress, so it's safe under privacy_mode.
  const handleShare = async () => {
    if (!recording) return;
    try {
      const path = await exportNoteMarkdown(recording.session_dir);
      try {
        await sharePaths([path]);
      } catch {
        await revealInFinder(path);
      }
      toast.success("Note exported", { description: "Markdown ready to share" });
    } catch (e) {
      console.error("share note:", e);
      toast.error("Could not export note", { description: String(e) });
    }
  };

  // GET-163: persist an edited title to `title.txt`. An empty value clears
  // it (falls back to the autoname/label). Optimistically updates local
  // state so the header reflects the change without a re-fetch.
  const handleRename = React.useCallback(
    async (next: string) => {
      if (!recording) return;
      const trimmed = next.trim();
      if ((recording.title ?? "") === trimmed) return;
      setRecording((prev) => (prev ? { ...prev, title: trimmed || null } : prev));
      try {
        await renameNote(recording.session_dir, trimmed);
      } catch (e) {
        console.error("rename_note:", e);
        toast.error("Could not rename note", { description: String(e) });
      }
    },
    [recording]
  );

  const handleReveal = () => {
    if (!recording) return;
    revealInFinder(recording.session_dir).catch((e) => {
      console.error("reveal_in_finder:", e);
      toast.error("Could not open Finder", { description: String(e) });
    });
  };

  const handleDelete = async () => {
    if (!recording) return;
    const noteName =
      recording.title?.trim() ||
      recording.suggested_title?.trim() ||
      recording.draft_name ||
      recording.label;
    const ok = await confirmDelete({
      title: "Delete this note?",
      description: `"${noteName}" — this removes the session folder and every file inside it (audio, transcript, notes). Cannot be undone.`,
      confirmLabel: "Delete note",
    });
    if (!ok) return;
    try {
      await deleteRecording(recording.session_dir);
      toast.success("Note deleted");
      navigate("/library");
    } catch (e) {
      console.error("delete_recording:", e);
      toast.error("Could not delete note", { description: String(e) });
    }
  };

  const isLegacyTranscript = React.useMemo(() => {
    if (!transcript) return false;
    return transcript.channels.some((c) => c.channel === "legacy");
  }, [transcript]);

  const handleReTranscribe = async () => {
    if (!recording) return;
    const ok = window.confirm(
      "Delete this note's transcript and every saved AI result, then re-transcribe with the latest pipeline?\n\nAudio files are not touched."
    );
    if (!ok) return;
    setReTranscribing(true);
    try {
      await clearRecordingArtifacts(recording.session_dir);
      setTranscript(null);
      await transcribeRecording(recording.session_dir);
      toast.success("Re-transcribing — fresh transcript on the way");
    } catch (e) {
      console.error("re-transcribe:", e);
      toast.error("Could not re-transcribe", { description: String(e) });
    } finally {
      setReTranscribing(false);
    }
  };

  // ---- Render guards ---------------------------------------------------

  if (notFound) {
    return (
      <CenteredPage>
        <h1 className="font-serif text-2xl font-medium">Note not found</h1>
        <p className="max-w-md text-sm text-muted-foreground">
          The note <span className="font-mono">{label}</span> does not exist in your
          recordings folder. It may have been deleted or renamed.
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
        <p className="text-sm text-muted-foreground">Loading note…</p>
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
  // While the note is being processed — transcribing locally/in the cloud,
  // or folding the markdown notes into the AI summary — lock the notes
  // editor and the record dock (greyed) so the user can see those fields
  // are in-flight and can't race the pipeline that consumes them.
  const isProcessing = isCurrentlyTranscribing || regenerating || summarizing;
  // GET-163: a user-set title wins; else the autoname suggestion; else the
  // timestamp label. The placeholder shown in the editable field is the
  // non-user fallback so clearing the field reveals what it'll fall back to.
  const fallbackTitle =
    recording.suggested_title?.trim() || recording.draft_name || recording.label;
  const title = recording.title?.trim() || fallbackTitle;
  const hasAudio = recording.mic_bytes !== null || recording.system_bytes !== null;
  // Record-dock state (GET-155): is THIS note the active capture?
  const isThisActive = recState.liveSessionDir === recording.session_dir;
  const isRecordingThis = recState.recording && isThisActive;
  const isPausedThis = recState.paused && isThisActive;
  const otherActive = (recState.recording || recState.paused) && !isThisActive;
  const dockElapsedLabel = formatElapsed(recState.elapsed);

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-8 pb-28">
      {/* Header: back + ⋯ */}
      <div data-drag="" className="flex select-none items-center justify-between">
        <Link
          to="/library"
          className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
        >
          <ArrowLeft className="h-3.5 w-3.5" />
          Library
        </Link>
        <div className="flex items-center gap-1.5">
          {recording.has_transcript ? (
            <>
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5"
                onClick={handleRegenerate}
                disabled={regenerating || isCurrentlyTranscribing}
              >
                {regenerating ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <RefreshCw className="h-3.5 w-3.5" />
                )}
                {summaryRun ? "Regenerate" : "Generate notes"}
              </Button>
              <FollowupEmailButton
                sessionDir={recording.session_dir}
                disabled={false}
              />
            </>
          ) : null}
          <NoteMenu
            hasTranscript={recording.has_transcript}
            hasSummary={summaryRun !== null}
            reTranscribing={reTranscribing || isCurrentlyTranscribing}
            onChat={() => setChatOpen(true)}
            onCopy={handleCopy}
            onShare={handleShare}
            onReTranscribe={handleReTranscribe}
            onReveal={handleReveal}
            onDelete={handleDelete}
          />
        </div>
      </div>

      {/* Title + chips */}
      <div className="space-y-3">
        <EditableTitle
          value={title}
          placeholder={fallbackTitle}
          onCommit={handleRename}
        />
        <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
          <Chip>{formatNoteDate(recording.created_at)}</Chip>
          <Chip>
            <UserIcon className="h-3 w-3" />
            Me
          </Chip>
          <FolderChip
            sessionDir={recording.session_dir}
            folder={recording.folder ?? null}
            onChange={(next) =>
              setRecording((prev) => (prev ? { ...prev, folder: next } : prev))
            }
          />
          <span className="font-mono">
            {formatDuration(Number(recording.duration_seconds))} ·{" "}
            {formatBytes(totalBytes)}
          </span>
          {recording.has_transcript ? (
            <Badge variant="accent" className="gap-1 text-2xs">
              <Sparkles className="h-3 w-3" />
              transcribed
            </Badge>
          ) : null}
        </div>
      </div>

      {/* Transcribing / transcribe-now (only when there's audio) */}
      {isCurrentlyTranscribing ? (
        <div
          className="flex items-center gap-2 text-sm text-muted-foreground"
          role="status"
          aria-live="polite"
        >
          <Loader2 className="h-4 w-4 animate-spin" />
          <span>{transcriber.progressLabel}</span>
        </div>
      ) : hasAudio && !recording.has_transcript ? (
        <div className="flex flex-col items-start gap-3 rounded-lg border border-dashed border-border bg-card/40 px-4 py-6">
          <div className="flex items-center gap-2">
            <FileText className="h-4 w-4 text-muted-foreground" />
            <p className="text-sm">This note has audio but no transcript yet.</p>
          </div>
          <Button onClick={handleTranscribe} className="gap-2">
            <Sparkles className="h-3.5 w-3.5" />
            Transcribe now
          </Button>
          <p className="text-xs text-muted-foreground">{transcriber.emptyStateHint}</p>
        </div>
      ) : null}

      {/* Your notes — a live markdown editor; autosaves to the note dir
          (GET-145/155). Feeds the on-stop summary (GET-147). */}
      <section className="space-y-2">
        <SectionLabel>Your notes</SectionLabel>
        <MarkdownNotesEditor
          sessionDir={recording.session_dir}
          elapsedSeconds={isRecordingThis ? recState.elapsed : 0}
          disabled={isProcessing}
        />
      </section>

      {/* Enhanced notes (the structured summary). GET-191: rendered muted
          until the user "keeps" it, so AI text reads as a draft to review
          rather than something they wrote. */}
      {summaryRun ? (
        <section className="space-y-2">
          <div className="flex items-center justify-between gap-2">
            <SectionLabel>Enhanced notes</SectionLabel>
            {enhancedNotesKept ? (
              <span className="inline-flex items-center gap-1 text-2xs text-muted-foreground">
                <Check className="h-3 w-3 text-emerald-500" />
                Kept
              </span>
            ) : (
              <button
                type="button"
                onClick={() => void keepEnhancedNotes()}
                title="Mark these AI-generated notes as reviewed and yours"
                className="inline-flex items-center gap-1 rounded-full border border-border bg-card px-2 py-0.5 text-2xs font-medium text-muted-foreground transition-colors hover:text-foreground"
              >
                <Check className="h-3 w-3" />
                Keep these
              </button>
            )}
          </div>
          <EnhancedNotesBody
            response={summaryRun.response}
            sessionDir={recording.session_dir}
            muted={!enhancedNotesKept}
          />
          {!enhancedNotesKept && (
            <p className="text-2xs text-muted-foreground/80">
              AI-generated from your transcript. Click any line to see the
              moment behind it. Review and keep to make it yours.
            </p>
          )}
        </section>
      ) : summarizing ? (
        <section className="space-y-2">
          <SectionLabel>Enhanced notes</SectionLabel>
          <div
            className="flex items-center gap-2 rounded-lg border border-dashed border-border bg-card/40 px-4 py-6 text-sm text-muted-foreground"
            role="status"
            aria-live="polite"
          >
            <Loader2 className="h-4 w-4 animate-spin" />
            <span>Generating enhanced notes…</span>
          </div>
        </section>
      ) : recording.has_transcript && !isCurrentlyTranscribing ? (
        <p className="text-sm text-muted-foreground">
          No enhanced notes yet — hit{" "}
          <span className="font-medium">Generate notes</span> above.
        </p>
      ) : null}

      {transcript ? <ParticipantCards transcript={transcript} /> : null}

      {isLegacyTranscript ? (
        <p className="rounded-md border border-amber-500/40 bg-amber-500/5 px-3 py-2 text-2xs text-amber-700 dark:text-amber-300">
          Legacy transcript (older pipeline). Use ⋯ → Re-transcribe to refresh it with
          the current pipeline. Audio is not touched.
        </p>
      ) : null}

      {/* Transcript dock — collapsible */}
      {recording.has_transcript ? (
        <Disclosure
          open={transcriptOpen}
          onToggle={() => setTranscriptOpen((v) => !v)}
          icon={FileText}
          label="Transcript & audio"
        >
          <div className="flex flex-col gap-4">
            {micPath ? (
              <AudioPlayer filePath={micPath} label="Mic" channel="mic" />
            ) : (
              <p className="text-xs text-muted-foreground">No mic track.</p>
            )}
            {systemPath ? <Separator /> : null}
            {systemPath ? (
              <AudioPlayer filePath={systemPath} label="System" channel="system" />
            ) : null}
            {transcriptLoading ? (
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
          </div>
        </Disclosure>
      ) : null}

      {/* Docked record panel (GET-155) — sticky to the bottom of the note */}
      <RecordDock
        recordingThis={isRecordingThis}
        pausedThis={isPausedThis}
        otherActive={otherActive}
        locked={isProcessing}
        liveTranscript={liveTranscriptEnabled}
        elapsedLabel={dockElapsedLabel}
        livePreview={isCapturingThis ? livePreview : ""}
        busy={recState.busy}
        canAsk={recording.has_transcript}
        onAsk={() => setChatOpen(true)}
        onRecord={() => void recState.start(recording.session_dir)}
        onStop={() => void recState.stop()}
        onPause={() => void recState.pause()}
        onResume={() => void recState.resume()}
      />

      <NoteChat
        sessionDir={recording.session_dir}
        open={chatOpen}
        onOpenChange={setChatOpen}
      />
    </div>
  );
}

/** Sticky bottom record control for a note (GET-155). While capturing,
 *  a live rolling-window transcript preview (GET-160) streams in above
 *  the controls; the final on-stop transcript remains the source of
 *  truth and lands in the "Transcript & audio" disclosure after Stop. */
function RecordDock({
  recordingThis,
  pausedThis,
  otherActive,
  locked,
  liveTranscript,
  elapsedLabel,
  livePreview,
  busy,
  canAsk,
  onAsk,
  onRecord,
  onStop,
  onPause,
  onResume,
}: {
  recordingThis: boolean;
  pausedThis: boolean;
  otherActive: boolean;
  /** Note is being processed (transcribe / summarize) — grey out and
   *  block the record controls until it settles. */
  locked: boolean;
  /** Live-transcript Beta opt-in. When off, the dock shows no live
   *  caption preview (the backend emits none either). */
  liveTranscript: boolean;
  elapsedLabel: string;
  livePreview: string;
  busy: boolean;
  canAsk: boolean;
  onAsk: () => void;
  onRecord: () => void;
  onStop: () => void;
  onPause: () => void;
  onResume: () => void;
}) {
  // When locked, the only thing the dock shows is a quiet "processing"
  // pill — no record/pause/stop, no Ask — so it's unmistakable the note
  // is busy. (Recording can't be in flight while transcribing anyway.)
  if (locked) {
    return (
      <div className="pointer-events-none sticky bottom-4 z-10 mt-2 flex flex-col items-center gap-2">
        <div
          className="pointer-events-none flex items-center gap-2 rounded-full border border-border bg-popover/95 px-4 py-2 text-sm text-muted-foreground opacity-70 shadow-lg backdrop-blur"
          role="status"
          aria-live="polite"
        >
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          Processing note…
        </div>
      </div>
    );
  }
  return (
    <div className="pointer-events-none sticky bottom-4 z-10 mt-2 flex flex-col items-center gap-2">
      {recordingThis && liveTranscript ? (
        <div
          className="pointer-events-auto max-w-xl rounded-2xl border border-border bg-popover/95 px-4 py-2 text-sm leading-relaxed text-muted-foreground shadow-lg backdrop-blur"
          aria-live="polite"
        >
          {livePreview ? (
            <span className="line-clamp-3">
              {livePreview}
              <span className="ml-0.5 animate-pulse">▍</span>
            </span>
          ) : (
            <span className="italic">Listening… live transcript will appear here.</span>
          )}
        </div>
      ) : null}
      <div className="pointer-events-auto flex items-center gap-2 rounded-full border border-border bg-popover/95 px-3 py-2 shadow-lg backdrop-blur">
        {canAsk ? (
          <button
            type="button"
            onClick={onAsk}
            className="inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-sm text-muted-foreground transition-colors hover:text-foreground"
          >
            <MessageCircleQuestion className="h-3.5 w-3.5" />
            Ask
          </button>
        ) : null}
        {canAsk ? <span className="h-4 w-px bg-border" /> : null}
        {recordingThis ? (
          <>
            <span className="flex items-center gap-1.5 px-1 font-mono text-sm tabular-nums">
              <span className="h-2 w-2 animate-pulse-record rounded-full bg-destructive" />
              {elapsedLabel}
            </span>
            <Button
              size="sm"
              variant="outline"
              className="gap-1.5"
              onClick={onPause}
              disabled={busy}
            >
              <Pause className="h-3.5 w-3.5" />
              Pause
            </Button>
            <Button
              size="sm"
              variant="destructive"
              className="gap-1.5"
              onClick={onStop}
              disabled={busy}
            >
              <Square className="h-3.5 w-3.5 fill-current" />
              Stop
            </Button>
          </>
        ) : pausedThis ? (
          <>
            <span className="flex items-center gap-1.5 px-1 font-mono text-sm tabular-nums text-muted-foreground">
              <span className="h-2 w-2 rounded-full bg-amber-500" />
              {elapsedLabel} paused
            </span>
            <Button size="sm" className="gap-1.5" onClick={onResume} disabled={busy}>
              <Play className="h-3.5 w-3.5 fill-current" />
              Resume
            </Button>
            <Button
              size="sm"
              variant="destructive"
              className="gap-1.5"
              onClick={onStop}
              disabled={busy}
            >
              <Square className="h-3.5 w-3.5 fill-current" />
              Stop
            </Button>
          </>
        ) : (
          <Button
            size="sm"
            className="gap-1.5"
            onClick={onRecord}
            disabled={busy || otherActive}
            title={
              otherActive ? "Another recording is in progress" : "Record into this note"
            }
          >
            <Mic className="h-3.5 w-3.5" />
            {otherActive ? "Recording elsewhere" : "Record"}
          </Button>
        )}
      </div>
    </div>
  );
}

function formatElapsed(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  const m = Math.floor(s / 60);
  const r = s % 60;
  return `${m}:${r.toString().padStart(2, "0")}`;
}

// ---- Small building blocks ---------------------------------------------

/** Inline-editable note title (GET-163). Renders as an unstyled input that
 *  looks like the heading; commits on blur or Enter, reverts on Escape.
 *  `value` is the resolved title (user title or fallback); `placeholder`
 *  is the non-user fallback shown when the field is emptied. */
function EditableTitle({
  value,
  placeholder,
  onCommit,
}: {
  value: string;
  placeholder: string;
  onCommit: (next: string) => void;
}) {
  const [draft, setDraft] = React.useState(value);
  const [editing, setEditing] = React.useState(false);

  // Keep the field in sync with upstream changes (e.g. autoname landing)
  // while the user isn't actively editing.
  React.useEffect(() => {
    if (!editing) setDraft(value);
  }, [value, editing]);

  const commit = () => {
    setEditing(false);
    // Only persist a real edit. Without this, focusing+blurring a note
    // whose displayed title is a fallback (a "Draft N" placeholder or an
    // autoname suggestion) would save that fallback as a user title and
    // freeze it — defeating the auto-rename once agents run.
    if (draft !== value) onCommit(draft);
  };

  return (
    <input
      type="text"
      aria-label="Note title"
      value={draft}
      placeholder={placeholder}
      onChange={(e) => setDraft(e.target.value)}
      onFocus={() => setEditing(true)}
      onBlur={commit}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.preventDefault();
          e.currentTarget.blur();
        } else if (e.key === "Escape") {
          setDraft(value);
          setEditing(false);
          e.currentTarget.blur();
        }
      }}
      className="w-full bg-transparent font-serif text-3xl font-medium tracking-tight outline-none placeholder:text-muted-foreground/50 focus:placeholder:text-transparent"
    />
  );
}

function Chip({ children }: { children: React.ReactNode }) {
  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-border bg-card px-2.5 py-1">
      {children}
    </span>
  );
}

function SectionLabel({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
      {children}
    </p>
  );
}

function Disclosure({
  open,
  onToggle,
  icon: Icon,
  label,
  children,
}: {
  open: boolean;
  onToggle: () => void;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-lg border border-border bg-card">
      <button
        type="button"
        onClick={onToggle}
        aria-expanded={open}
        className="flex w-full items-center gap-2 px-3 py-2.5 text-sm font-medium text-foreground"
      >
        <ChevronRight
          className={
            "h-4 w-4 text-muted-foreground transition-transform " +
            (open ? "rotate-90" : "")
          }
        />
        <Icon className="h-4 w-4 text-muted-foreground" />
        {label}
      </button>
      {open ? <div className="border-t border-border px-3 py-4">{children}</div> : null}
    </section>
  );
}

function NoteMenu({
  hasTranscript,
  hasSummary,
  reTranscribing,
  onChat,
  onCopy,
  onShare,
  onReTranscribe,
  onReveal,
  onDelete,
}: {
  hasTranscript: boolean;
  hasSummary: boolean;
  reTranscribing: boolean;
  onChat: () => void;
  onCopy: () => void;
  onShare: () => void;
  onReTranscribe: () => void;
  onReveal: () => void;
  onDelete: () => void;
}) {
  const [open, setOpen] = React.useState(false);
  const run = (fn: () => void) => () => {
    setOpen(false);
    fn();
  };
  return (
    <div className="relative">
      <Button
        type="button"
        variant="ghost"
        size="sm"
        aria-label="More actions"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <MoreHorizontal className="h-4 w-4" />
      </Button>
      {open ? (
        <>
          {/* click-away */}
          <button
            type="button"
            aria-hidden="true"
            tabIndex={-1}
            className="fixed inset-0 z-10 cursor-default"
            onClick={() => setOpen(false)}
          />
          <div
            role="menu"
            className="absolute right-0 top-full z-20 mt-1 w-52 overflow-hidden rounded-md border border-border bg-popover py-1 text-sm shadow-lg"
          >
            {hasTranscript ? (
              <MenuItem icon={MessageCircleQuestion} onClick={run(onChat)}>
                Chat with this note
              </MenuItem>
            ) : null}
            {hasSummary ? (
              <MenuItem icon={Copy} onClick={run(onCopy)}>
                Copy notes
              </MenuItem>
            ) : null}
            <MenuItem icon={Share} onClick={run(onShare)}>
              Share / export
            </MenuItem>
            {hasTranscript ? (
              <MenuItem
                icon={RefreshCw}
                onClick={run(onReTranscribe)}
                disabled={reTranscribing}
              >
                Re-transcribe
              </MenuItem>
            ) : null}
            <MenuItem icon={FileText} onClick={run(onReveal)}>
              Reveal in Finder
            </MenuItem>
            <MenuItem icon={Trash2} onClick={run(onDelete)} destructive>
              Delete note
            </MenuItem>
          </div>
        </>
      ) : null}
    </div>
  );
}

function MenuItem({
  icon: Icon,
  onClick,
  children,
  destructive,
  disabled,
}: {
  icon: React.ComponentType<{ className?: string }>;
  onClick: () => void;
  children: React.ReactNode;
  destructive?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="menuitem"
      onClick={onClick}
      disabled={disabled}
      className={
        "flex w-full items-center gap-2 px-3 py-1.5 text-left transition-colors disabled:opacity-50 " +
        (destructive
          ? "text-destructive hover:bg-destructive/10"
          : "text-foreground hover:bg-accent hover:text-accent-foreground")
      }
    >
      <Icon className="h-3.5 w-3.5" />
      {children}
    </button>
  );
}

function formatNoteDate(createdAt: string | null): string {
  if (!createdAt) return "Today";
  const d = new Date(createdAt);
  if (Number.isNaN(d.getTime())) return "Today";
  return d.toLocaleDateString([], { month: "short", day: "numeric", year: "numeric" });
}

function CenteredPage({ children }: { children: React.ReactNode }) {
  return (
    <div className="mx-auto flex h-full w-full max-w-md flex-col items-center justify-center gap-4 px-8 py-16 text-center">
      {children}
    </div>
  );
}
