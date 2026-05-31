import * as React from "react";
import {
  Captions,
  Download,
  FileText,
  Loader2,
  Save,
  Search,
  Undo2,
  Users,
  X,
} from "lucide-react";
import { showSaveDialog } from "@/shared/lib/ipc";
import { writeTextFileFromBrowser } from "@/shared/lib/ipc";
import { useVirtualizer } from "@tanstack/react-virtual";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { cn } from "@/shared/lib/utils";
import {
  buildConversation,
  type ConversationRow,
  otherSpeakerLabels,
} from "@/shared/lib/conversation";
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

/**
 * Number of visible segments above which the channel switches from a
 * flat `<ol>` to a virtualised list. v2 finding 062 / GET-97. Below
 * the threshold the flat list keeps the simpler DOM + spell-check
 * affordances; above, the virtualiser caps off-screen render cost so
 * a 4-hour meeting (~5000 segments) doesn't blow the WebView memory
 * budget.
 */
const VIRTUALIZATION_THRESHOLD = 200;

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
 * Editable transcript surface. The mic and system channels are merged
 * into one chronological, speaker-labelled conversation ("You" for the
 * note-taker, "Speaker 1/2/3…" for each diarized participant); every
 * segment is individually editable while its timestamp stays anchored.
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
      await writeTextFileFromBrowser(path, content);
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

      <ConversationEditor
        working={working}
        query={query}
        onSegmentChange={updateSegment}
      />

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

interface ConversationEditorProps {
  working: SessionTranscript;
  query: string;
  onSegmentChange: (channelIndex: number, segmentIndex: number, text: string) => void;
}

/**
 * The whole session as one chronological, speaker-labelled conversation:
 * the user's mic turns ("You") interleaved by time with each diarized
 * participant ("Speaker 1/2/3…"). This is the same shape the AI agents
 * read (see `buildConversation` / the Rust `to_labeled_dialogue`), so the
 * transcript a person edits matches the dialogue the summary reasons over.
 */
function ConversationEditor({ working, query, onSegmentChange }: ConversationEditorProps) {
  const rows = React.useMemo(() => buildConversation(working), [working]);
  const speakers = React.useMemo(() => otherSpeakerLabels(rows), [rows]);

  const filtered = React.useMemo(() => {
    if (query.trim().length === 0) return rows;
    return rows.filter((r) => segmentMatches(r.segment, query));
  }, [rows, query]);

  const visibleCount = filtered.length;

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center gap-2 text-xs font-medium text-foreground">
        <Users className="h-3.5 w-3.5 text-muted-foreground" />
        Conversation
        <span className="text-2xs font-normal text-muted-foreground">
          {speakers.length === 0
            ? "· you only"
            : `· you + ${speakers.length} ${
                speakers.length === 1 ? "speaker" : "speakers"
              }`}
        </span>
        {query.trim().length > 0 && (
          <span
            className={cn(
              "text-2xs tabular-nums",
              visibleCount === 0 ? "text-muted-foreground/60" : "text-muted-foreground"
            )}
          >
            · {visibleCount}/{rows.length}
          </span>
        )}
      </div>

      {visibleCount === 0 ? (
        <p className="text-2xs text-muted-foreground">No segments match.</p>
      ) : filtered.length > VIRTUALIZATION_THRESHOLD ? (
        <VirtualConversationList
          filtered={filtered}
          query={query}
          onSegmentChange={onSegmentChange}
        />
      ) : (
        <ol className="flex flex-col gap-2">
          {filtered.map((row, i) => (
            <SegmentRow
              key={`${row.channelIndex}-${row.segmentIndex}-${row.segment.start_seconds}`}
              segment={row.segment}
              index={i}
              channel={row.channelId}
              query={query}
              speakerLabel={row.label}
              pillClass={row.pillClass}
              onChange={(text) =>
                onSegmentChange(row.channelIndex, row.segmentIndex, text)
              }
            />
          ))}
        </ol>
      )}
    </section>
  );
}

function sameSession(a: SessionTranscript, b: SessionTranscript): boolean {
  if (a.channels.length !== b.channels.length) return false;
  for (let i = 0; i < a.channels.length; i++) {
    const ca = a.channels[i];
    const cb = b.channels[i];
    if (!ca || !cb) return false;
    if (!sameChannel(ca, cb)) return false;
  }
  return true;
}

function sameChannel(a: ChannelTranscript, b: ChannelTranscript): boolean {
  if (a.channel !== b.channel) return false;
  if (a.language !== b.language) return false;
  if (a.segments.length !== b.segments.length) return false;
  for (let i = 0; i < a.segments.length; i++) {
    const sa = a.segments[i];
    const sb = b.segments[i];
    if (!sa || !sb) return false;
    if (sa.text !== sb.text) return false;
    if (sa.start_seconds !== sb.start_seconds) return false;
    if (sa.end_seconds !== sb.end_seconds) return false;
  }
  return true;
}

function pathLeaf(path: string): string {
  return path.split("/").filter(Boolean).pop() ?? path;
}

interface VirtualConversationListProps {
  filtered: ConversationRow[];
  query: string;
  onSegmentChange: (channelIndex: number, segmentIndex: number, text: string) => void;
}

/**
 * Virtualised list for very long conversations. Uses @tanstack/react-
 * virtual with dynamic measurement — segments auto-grow to match
 * their textarea height, and the virtualiser observes each rendered
 * row so the absolute positions stay accurate as the user edits. We
 * keep an overscan of 8 rows above and below the viewport so
 * scrolling feels instant on common gesture distances.
 *
 * Height is capped at 1200px (vs. the whole transcript) so the
 * scrolling lives inside the card; the parent route's ScrollArea
 * handles document-level navigation. v2 finding 062 / GET-97.
 */
function VirtualConversationList({
  filtered,
  query,
  onSegmentChange,
}: VirtualConversationListProps) {
  const parentRef = React.useRef<HTMLDivElement>(null);
  const rowVirtualizer = useVirtualizer({
    count: filtered.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 56,
    overscan: 8,
    measureElement:
      typeof window !== "undefined" && "ResizeObserver" in window
        ? (el) => el.getBoundingClientRect().height
        : undefined,
  });

  return (
    <div
      ref={parentRef}
      className="relative max-h-[1200px] overflow-y-auto rounded-md border border-border bg-card/40"
      data-virtualized="true"
    >
      <ol
        style={{ height: `${rowVirtualizer.getTotalSize()}px` }}
        className="relative w-full"
      >
        {rowVirtualizer.getVirtualItems().map((virtualRow) => {
          const row = filtered[virtualRow.index];
          if (!row) return null;
          return (
            <li
              key={`${row.channelIndex}-${row.segmentIndex}-${row.segment.start_seconds}`}
              data-index={virtualRow.index}
              ref={rowVirtualizer.measureElement}
              className="absolute left-0 top-0 w-full px-1.5 py-1"
              style={{ transform: `translateY(${virtualRow.start}px)` }}
            >
              <SegmentRow
                segment={row.segment}
                index={virtualRow.index}
                channel={row.channelId}
                query={query}
                speakerLabel={row.label}
                pillClass={row.pillClass}
                onChange={(text) =>
                  onSegmentChange(row.channelIndex, row.segmentIndex, text)
                }
              />
            </li>
          );
        })}
      </ol>
    </div>
  );
}
