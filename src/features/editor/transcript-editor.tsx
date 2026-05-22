import * as React from "react";
import { Loader2, Save, Undo2 } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { saveTranscript } from "@/shared/lib/ipc";
import type { Transcript } from "@/shared/types/Transcript";
import type { TranscriptSegment } from "@/shared/types/TranscriptSegment";

import { SegmentRow } from "./segment-row";

interface Props {
  sessionDir: string;
  initial: Transcript;
  onSaved: (next: Transcript) => void;
}

/**
 * Editable transcript surface. Keeps a working copy of the transcript
 * in local state; the dirty flag is recomputed from a shallow segment
 * comparison so undo (Discard) cleanly resets to the loaded version.
 *
 * Save writes the JSON back to disk atomically via the `save_transcript`
 * Tauri command and bubbles the saved transcript up so the parent can
 * update its baseline.
 */
export function TranscriptEditor({ sessionDir, initial, onSaved }: Props) {
  const [working, setWorking] = React.useState<Transcript>(initial);
  const [saving, setSaving] = React.useState(false);

  // Reset the working copy when the upstream baseline changes (e.g. a
  // re-transcription just landed).
  React.useEffect(() => {
    setWorking(initial);
  }, [initial]);

  const dirty = React.useMemo(
    () => !sameTranscript(working, initial),
    [working, initial]
  );

  // Warn the user about unsaved edits on window close. React Router's
  // own navigation guard is more involved — for now we cover the
  // window close case.
  React.useEffect(() => {
    if (!dirty) return;
    const handler = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = "";
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [dirty]);

  const updateSegment = React.useCallback((index: number, text: string) => {
    setWorking((cur) => {
      const next: TranscriptSegment[] = cur.segments.map((s, i) =>
        i === index ? { ...s, text } : s
      );
      return { ...cur, segments: next };
    });
  }, []);

  const handleDiscard = () => {
    setWorking(initial);
    toast.message("Changes discarded");
  };

  const handleSave = async () => {
    setSaving(true);
    try {
      await saveTranscript(sessionDir, working);
      onSaved(working);
      toast.success("Transcript saved");
    } catch (e) {
      console.error("save_transcript:", e);
      toast.error("Could not save transcript", { description: String(e) });
    } finally {
      setSaving(false);
    }
  };

  if (working.segments.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        Whisper returned no segments for this audio. Try re-transcribing or
        re-recording.
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-4">
      <header className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Transcript
          {dirty && (
            <span
              className="rounded-full bg-accent px-2 py-0.5 text-2xs font-medium text-foreground"
              aria-live="polite"
            >
              unsaved changes
            </span>
          )}
        </div>
        {working.language && (
          <span className="font-mono text-2xs text-muted-foreground">
            {working.language}
          </span>
        )}
      </header>

      <ol className="flex flex-col gap-2">
        {working.segments.map((segment, i) => (
          <SegmentRow
            key={`${i}-${segment.start_seconds}`}
            segment={segment}
            index={i}
            onChange={(text) => updateSegment(i, text)}
          />
        ))}
      </ol>

      <footer className="flex items-center justify-end gap-2 border-t border-border pt-3">
        <Button
          variant="ghost"
          onClick={handleDiscard}
          disabled={!dirty || saving}
          className="gap-2"
        >
          <Undo2 className="h-3.5 w-3.5" />
          Discard
        </Button>
        <Button onClick={handleSave} disabled={!dirty || saving} className="gap-2">
          {saving ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Save className="h-3.5 w-3.5" />
          )}
          {saving ? "Saving…" : "Save changes"}
        </Button>
      </footer>
    </div>
  );
}

function sameTranscript(a: Transcript, b: Transcript): boolean {
  if (a.language !== b.language) return false;
  if (a.segments.length !== b.segments.length) return false;
  for (let i = 0; i < a.segments.length; i++) {
    if (a.segments[i].text !== b.segments[i].text) return false;
    if (a.segments[i].start_seconds !== b.segments[i].start_seconds) return false;
    if (a.segments[i].end_seconds !== b.segments[i].end_seconds) return false;
  }
  return true;
}
