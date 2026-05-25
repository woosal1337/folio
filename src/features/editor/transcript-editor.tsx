import * as React from "react";
import {
  Captions,
  Download,
  FileText,
  Headphones,
  Loader2,
  Mic,
  Save,
  Search,
  Undo2,
  X,
} from "lucide-react";
import { save as showSaveDialog } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { cn } from "@/shared/lib/utils";
import { saveTranscript } from "@/shared/lib/ipc";
import type { ChannelTranscript } from "@/shared/types/ChannelTranscript";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";

import { SegmentRow } from "./segment-row";
import {
  type ExportFormat,
  extensionFor,
  renderTranscript,
  segmentMatches,
} from "./transcript-export";

interface Props {
  sessionDir: string;
  initial: SessionTranscript;
  onSaved: (next: SessionTranscript) => void;
}

const EXPORT_FORMATS: Array<{
  id: ExportFormat;
  label: string;
  Icon: React.ComponentType<{ className?: string }>;
  description: string;
}> = [
  {
    id: "srt",
    label: "SRT",
    Icon: Captions,
    description: "SubRip caption file (.srt)",
  },
  {
    id: "vtt",
    label: "WebVTT",
    Icon: Captions,
    description: "Web Video Text Tracks (.vtt)",
  },
  {
    id: "txt",
    label: "Plain text",
    Icon: FileText,
    description: "Timestamped plain text (.txt)",
  },
];

/**
 * Editable multi-channel transcript surface. Each channel ("You",
 * "Others") renders as its own labelled section; segments inside each
 * are individually editable while their timestamps stay anchored.
 *
 * v2 roadmap finding 102 (GET-114) adds:
 *  - a live search box that filters segments by case-insensitive
 *    substring match and highlights the match in-line
 *  - a click-to-seek timestamp on every segment, which fires a
 *    `attune:seek-audio` window event the AudioPlayer subscribes to
 *  - SRT / WebVTT / plain-text-timestamps export via a native save
 *    dialog
 */
export function TranscriptEditor({ sessionDir, initial, onSaved }: Props) {
  const [working, setWorking] = React.useState<SessionTranscript>(initial);
  const [saving, setSaving] = React.useState(false);
  const [exporting, setExporting] = React.useState<ExportFormat | null>(null);
  const [query, setQuery] = React.useState("");

  React.useEffect(() => {
    setWorking(initial);
  }, [initial]);

  const dirty = React.useMemo(() => !sameSession(working, initial), [working, initial]);

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

  const handleExport = async (format: ExportFormat) => {
    if (dirty) {
      toast.message("Save your edits first", {
        description: "Export uses the saved transcript on disk.",
      });
      return;
    }
    setExporting(format);
    try {
      const content = renderTranscript(working, format);
      const ext = extensionFor(format);
      // Suggest a filename derived from the session directory's leaf
      // name (the recording label), with the right extension.
      const leaf = sessionDir.split("/").filter(Boolean).pop() ?? "transcript";
      const path = await showSaveDialog({
        defaultPath: `${leaf}.${ext}`,
        filters: [{ name: format.toUpperCase(), extensions: [ext] }],
      });
      if (!path) return; // user cancelled
      await writeTextFile(path, content);
      toast.success(`Exported to ${pathLeaf(path)}`);
    } catch (e) {
      console.error("export transcript:", e);
      toast.error("Could not export transcript", { description: String(e) });
    } finally {
      setExporting(null);
    }
  };

  const totalSegments = working.channels.reduce(
    (acc, ch) => acc + ch.segments.length,
    0
  );

  const matchCount = React.useMemo(() => {
    if (query.trim().length === 0) return 0;
    return working.channels.reduce(
      (acc, ch) => acc + ch.segments.filter((s) => segmentMatches(s, query)).length,
      0
    );
  }, [working, query]);

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
      <header className="flex flex-wrap items-center justify-between gap-3">
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
        <div className="relative flex w-full max-w-xs items-center">
          <Search
            className="pointer-events-none absolute left-2.5 h-3.5 w-3.5 text-muted-foreground"
            aria-hidden
          />
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search transcript…"
            aria-label="Search transcript"
            className="h-8 pl-8 pr-8 text-sm"
          />
          {query.length > 0 && (
            <button
              type="button"
              onClick={() => setQuery("")}
              aria-label="Clear search"
              className="absolute right-2 inline-flex h-4 w-4 items-center justify-center rounded text-muted-foreground hover:text-foreground"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          )}
        </div>
      </header>

      {query.trim().length > 0 && (
        <p className="text-2xs tabular-nums text-muted-foreground" aria-live="polite">
          {matchCount === 0
            ? "No segments match"
            : `${matchCount} segment${matchCount === 1 ? "" : "s"} match`}
        </p>
      )}

      {working.channels.map((channel, ci) => (
        <ChannelEditor
          key={channel.channel}
          channel={channel}
          query={query}
          onSegmentChange={(si, text) => updateSegment(ci, si, text)}
        />
      ))}

      <footer className="flex flex-wrap items-center justify-end gap-2 border-t border-border pt-3">
        <div className="mr-auto flex flex-wrap items-center gap-1.5">
          {EXPORT_FORMATS.map((f) => {
            const Icon = f.Icon;
            const busy = exporting === f.id;
            return (
              <Button
                key={f.id}
                variant="outline"
                size="sm"
                disabled={busy || saving}
                onClick={() => handleExport(f.id)}
                title={f.description}
                className="gap-1.5"
              >
                {busy ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Icon className="h-3.5 w-3.5" />
                )}
                <span className="text-xs">{f.label}</span>
                {!busy && <Download className="h-3 w-3 opacity-60" />}
              </Button>
            );
          })}
        </div>
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
  query: string;
  onSegmentChange: (segmentIndex: number, text: string) => void;
}

function ChannelEditor({ channel, query, onSegmentChange }: ChannelEditorProps) {
  const meta = channelLabel(channel.channel);
  const filtered = React.useMemo(() => {
    if (query.trim().length === 0) {
      return channel.segments.map((segment, index) => ({ segment, index }));
    }
    return channel.segments
      .map((segment, index) => ({ segment, index }))
      .filter(({ segment }) => segmentMatches(segment, query));
  }, [channel.segments, query]);

  const visibleCount = filtered.length;

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-xs font-medium text-foreground">
          <meta.Icon className="h-3.5 w-3.5 text-muted-foreground" />
          {meta.label}
          <span className="text-2xs font-normal text-muted-foreground">{meta.sub}</span>
          {query.trim().length > 0 && (
            <span
              className={cn(
                "text-2xs tabular-nums",
                visibleCount === 0
                  ? "text-muted-foreground/60"
                  : "text-muted-foreground"
              )}
            >
              · {visibleCount}/{channel.segments.length}
            </span>
          )}
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
      ) : visibleCount === 0 ? (
        <p className="text-2xs text-muted-foreground">No segments match.</p>
      ) : (
        <ol className="flex flex-col gap-2">
          {filtered.map(({ segment, index }) => (
            <SegmentRow
              key={`${index}-${segment.start_seconds}`}
              segment={segment}
              index={index}
              channel={channel.channel}
              query={query}
              onChange={(text) => onSegmentChange(index, text)}
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

function pathLeaf(path: string): string {
  return path.split("/").filter(Boolean).pop() ?? path;
}
