import * as React from "react";
import { Headphones, Loader2, Mic, Save, Undo2 } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { saveTranscript } from "@/shared/lib/ipc";
import type { ChannelTranscript } from "@/shared/types/ChannelTranscript";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";

import { SegmentRow } from "./segment-row";

interface Props {
  sessionDir: string;
  initial: SessionTranscript;
  onSaved: (next: SessionTranscript) => void;
}

/**
 * Editable multi-channel transcript surface. Each channel ("You",
 * "Others") renders as its own labelled section; segments inside each
 * are individually editable while their timestamps stay anchored.
 *
 * The working copy lives in local state. The dirty flag is recomputed
 * by deep-comparing channels and segments, so Discard cleanly resets
 * to the loaded version. Save writes the whole `SessionTranscript`
 * back via the `save_transcript` Tauri command.
 */
export function TranscriptEditor({ sessionDir, initial, onSaved }: Props) {
  const [working, setWorking] = React.useState<SessionTranscript>(initial);
  const [saving, setSaving] = React.useState(false);

  // Reset the working copy when the upstream baseline changes (e.g. a
  // re-transcription just landed).
  React.useEffect(() => {
    setWorking(initial);
  }, [initial]);

  const dirty = React.useMemo(() => !sameSession(working, initial), [working, initial]);

  // Warn the user about unsaved edits on window close.
  React.useEffect(() => {
    if (!dirty) return;
    const handler = (e: BeforeUnloadEvent) => {
      e.preventDefault();
      e.returnValue = "";
    };
    window.addEventListener("beforeunload", handler);
    return () => window.removeEventListener("beforeunload", handler);
  }, [dirty]);

  const updateSegment = React.useCallback(
    (channelIndex: number, segmentIndex: number, text: string) => {
      setWorking((cur) => ({
        ...cur,
        channels: cur.channels.map((channel, ci) => {
          if (ci !== channelIndex) return channel;
          return {
            ...channel,
            segments: channel.segments.map((seg, si) =>
              si === segmentIndex ? { ...seg, text } : seg
            ),
          };
        }),
      }));
    },
    []
  );

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

  const totalSegments = working.channels.reduce(
    (acc, ch) => acc + ch.segments.length,
    0
  );
  if (totalSegments === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        Whisper returned no segments for this audio. Try re-transcribing or
        re-recording.
      </p>
    );
  }

  return (
    <div className="flex flex-col gap-5">
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
      </header>

      {working.channels.map((channel, ci) => (
        <ChannelEditor
          key={channel.channel}
          channel={channel}
          onSegmentChange={(si, text) => updateSegment(ci, si, text)}
        />
      ))}

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

interface ChannelEditorProps {
  channel: ChannelTranscript;
  onSegmentChange: (segmentIndex: number, text: string) => void;
}

function ChannelEditor({ channel, onSegmentChange }: ChannelEditorProps) {
  const meta = channelLabel(channel.channel);

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-medium text-foreground">
          <meta.Icon className="h-3.5 w-3.5 text-muted-foreground" />
          {meta.label}
          <span className="text-2xs font-normal text-muted-foreground">{meta.sub}</span>
        </div>
        {channel.language && (
          <span className="font-mono text-2xs text-muted-foreground">
            {channel.language}
          </span>
        )}
      </div>

      {channel.segments.length === 0 ? (
        <p className="text-2xs text-muted-foreground">
          No speech detected on this channel.
        </p>
      ) : (
        <ol className="flex flex-col gap-2">
          {channel.segments.map((segment, i) => (
            <SegmentRow
              key={`${i}-${segment.start_seconds}`}
              segment={segment}
              index={i}
              onChange={(text) => onSegmentChange(i, text)}
            />
          ))}
        </ol>
      )}
    </section>
  );
}

interface ChannelMeta {
  label: string;
  sub: string;
  Icon: React.ComponentType<{ className?: string }>;
}

function channelLabel(channel: string): ChannelMeta {
  switch (channel) {
    case "mic":
      return { label: "You", sub: "(microphone)", Icon: Mic };
    case "system":
      return { label: "Others", sub: "(system audio)", Icon: Headphones };
    default:
      return { label: channel, sub: "", Icon: Mic };
  }
}

function sameSession(a: SessionTranscript, b: SessionTranscript): boolean {
  if (a.channels.length !== b.channels.length) return false;
  for (let i = 0; i < a.channels.length; i++) {
    if (!sameChannel(a.channels[i], b.channels[i])) return false;
  }
  return true;
}

function sameChannel(a: ChannelTranscript, b: ChannelTranscript): boolean {
  if (a.channel !== b.channel) return false;
  if (a.language !== b.language) return false;
  if (a.segments.length !== b.segments.length) return false;
  for (let i = 0; i < a.segments.length; i++) {
    if (a.segments[i].text !== b.segments[i].text) return false;
    if (a.segments[i].start_seconds !== b.segments[i].start_seconds) return false;
    if (a.segments[i].end_seconds !== b.segments[i].end_seconds) return false;
  }
  return true;
}
