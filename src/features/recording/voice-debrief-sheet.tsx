import * as React from "react";
import { Mic, MicOff, X } from "lucide-react";
import { toast } from "sonner";

import { Dialog, DialogContent, DialogDescription, DialogTitle } from "@/shared/ui/dialog";
import { saveDebrief } from "@/shared/lib/ipc";

const MAX_SECONDS = 20;

interface Props {
  sessionDir: string | null;
  onClose: () => void;
}

/**
 * Voice-debrief on Stop. v2 finding 027 / GET-53.
 *
 * Mounts after the user hits Stop and the meeting has just been saved.
 * Asks "anything to capture before this fades?" and records up to 20s
 * of mic via `MediaRecorder`. On done, we save the blob next to the
 * meeting as `debrief.webm` via `save_debrief`. The downstream
 * extract-tasks / extract-memories agents pick it up alongside the
 * main transcript when the recording is opened.
 *
 * Default-OFF; this component only mounts when
 * `settings.voice_debrief_enabled` is true.
 */
export function VoiceDebriefSheet({ sessionDir, onClose }: Props) {
  const open = sessionDir !== null;
  const [recording, setRecording] = React.useState(false);
  const [elapsed, setElapsed] = React.useState(0);
  const [saving, setSaving] = React.useState(false);
  const recorderRef = React.useRef<MediaRecorder | null>(null);
  const chunksRef = React.useRef<Blob[]>([]);
  const streamRef = React.useRef<MediaStream | null>(null);
  const tickerRef = React.useRef<number | null>(null);

  const cleanup = React.useCallback(() => {
    if (tickerRef.current !== null) {
      window.clearInterval(tickerRef.current);
      tickerRef.current = null;
    }
    if (streamRef.current) {
      for (const track of streamRef.current.getTracks()) track.stop();
      streamRef.current = null;
    }
    recorderRef.current = null;
    chunksRef.current = [];
    setElapsed(0);
    setRecording(false);
  }, []);

  React.useEffect(() => {
    if (!open) cleanup();
    return cleanup;
  }, [open, cleanup]);

  const start = React.useCallback(async () => {
    if (!sessionDir) return;
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;
      const mime = MediaRecorder.isTypeSupported("audio/webm;codecs=opus")
        ? "audio/webm;codecs=opus"
        : "audio/webm";
      const rec = new MediaRecorder(stream, { mimeType: mime });
      chunksRef.current = [];
      rec.ondataavailable = (e) => {
        if (e.data && e.data.size > 0) chunksRef.current.push(e.data);
      };
      rec.onstop = async () => {
        const blob = new Blob(chunksRef.current, { type: mime });
        try {
          setSaving(true);
          const buf = await blob.arrayBuffer();
          await saveDebrief(sessionDir, "debrief.webm", new Uint8Array(buf));
          toast.success("Debrief saved", {
            description: "Will be transcribed alongside the main recording.",
          });
        } catch (err) {
          console.error("save_debrief:", err);
          toast.error("Could not save debrief", { description: String(err) });
        } finally {
          setSaving(false);
          cleanup();
          onClose();
        }
      };
      rec.start();
      recorderRef.current = rec;
      setRecording(true);
      setElapsed(0);
      const startedAt = Date.now();
      tickerRef.current = window.setInterval(() => {
        const t = Math.floor((Date.now() - startedAt) / 1000);
        setElapsed(t);
        if (t >= MAX_SECONDS) {
          rec.stop();
          if (tickerRef.current !== null) {
            window.clearInterval(tickerRef.current);
            tickerRef.current = null;
          }
        }
      }, 200);
    } catch (err) {
      console.error("mic permission:", err);
      toast.error("Microphone access denied", { description: String(err) });
      cleanup();
    }
  }, [sessionDir, cleanup, onClose]);

  const stop = React.useCallback(() => {
    const rec = recorderRef.current;
    if (rec && rec.state !== "inactive") rec.stop();
  }, []);

  return (
    <Dialog open={open} onOpenChange={(v) => !v && (stop(), onClose())}>
      <DialogContent className="max-w-md">
        <div className="flex flex-col items-center gap-5 py-2 text-center">
          <DialogTitle className="font-serif text-xl font-medium">
            Anything to capture before this fades?
          </DialogTitle>
          <DialogDescription className="text-sm text-muted-foreground">
            Tap the mic and speak for up to {MAX_SECONDS}s. The clip rides into the
            same extract-tasks / extract-memories pass as the meeting.
          </DialogDescription>
          <button
            type="button"
            onClick={recording ? stop : start}
            disabled={saving}
            aria-label={recording ? "Stop debrief" : "Start debrief"}
            className="group flex h-20 w-20 items-center justify-center rounded-full bg-primary text-primary-foreground transition-transform hover:scale-105 disabled:opacity-60"
          >
            {recording ? <MicOff className="h-8 w-8" /> : <Mic className="h-8 w-8" />}
          </button>
          {recording ? (
            <p className="font-mono text-2xs uppercase tracking-wider text-muted-foreground">
              recording · {elapsed}s / {MAX_SECONDS}s
            </p>
          ) : saving ? (
            <p className="font-mono text-2xs uppercase tracking-wider text-muted-foreground">
              saving…
            </p>
          ) : (
            <p className="text-xs text-muted-foreground">Press Esc to skip.</p>
          )}
        </div>
        <button
          type="button"
          onClick={() => {
            stop();
            onClose();
          }}
          aria-label="Skip debrief"
          className="absolute right-3 top-3 rounded-full p-1 text-muted-foreground hover:text-foreground"
        >
          <X className="h-4 w-4" />
        </button>
      </DialogContent>
    </Dialog>
  );
}
