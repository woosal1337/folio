import * as React from "react";
import { Check, Loader2, Pencil, X } from "lucide-react";
import { toast } from "sonner";

import { cn } from "@/shared/lib/utils";
import {
  confirmSessionSpeaker,
  rejectSessionSpeaker,
  renameSessionSpeaker,
} from "@/shared/lib/ipc";
import type { ConversationSpeaker } from "@/shared/lib/conversation";
import type { SpeakerLabel } from "@/shared/types/SpeakerLabel";

interface Props {
  sessionDir: string;
  /** Distinct system speakers in the conversation, in "Speaker N" order. */
  speakers: ConversationSpeaker[];
  /** Backend label per cluster — carries `has_embedding` / `auto_named`. */
  labelsByCluster: Map<number, SpeakerLabel>;
  /** Called with the fresh label set after a successful rename. */
  onRenamed: (labels: SpeakerLabel[]) => void;
}

/**
 * Rename strip above the transcript. Each diarized speaker is a chip; click
 * the pencil to give the voice a real name. The name sticks to this
 * recording and — when the cluster carries a voice embedding — teaches the
 * cross-recording registry so the same speaker is auto-detected next time
 * (GET-189). Clusters with too little audio to embed can still be named
 * here, but won't be remembered; the chip says so.
 *
 * A medium-confidence registry match surfaces as an inline "· <name>?"
 * prompt with confirm / reject: confirming adds this voice as another
 * exemplar (moving the identity toward silent auto-naming), rejecting
 * records a "not this person" vote so the suggestion stops recurring.
 */
export function SpeakerLegend({
  sessionDir,
  speakers,
  labelsByCluster,
  onRenamed,
}: Props) {
  const [editing, setEditing] = React.useState<number | null>(null);
  const [draft, setDraft] = React.useState("");
  const [saving, setSaving] = React.useState(false);
  const [acting, setActing] = React.useState<number | null>(null);

  if (speakers.length === 0) return null;

  const respond = async (cluster: number, accept: boolean) => {
    const suggested = labelsByCluster.get(cluster)?.suggested_name;
    setActing(cluster);
    try {
      const labels = accept
        ? await confirmSessionSpeaker(sessionDir, cluster)
        : await rejectSessionSpeaker(sessionDir, cluster);
      onRenamed(labels);
      if (accept) {
        toast.success(`Confirmed ${suggested ?? "speaker"}`, {
          description: "Future recordings are likelier to recognise this voice.",
        });
      } else {
        toast.message("Got it — won't suggest that name again");
      }
    } catch (e) {
      console.error("speaker suggestion:", e);
      toast.error("Could not update speaker", { description: String(e) });
    } finally {
      setActing(null);
    }
  };

  const begin = (s: ConversationSpeaker) => {
    setEditing(s.cluster);
    setDraft(s.named ? s.label : "");
  };

  const cancel = () => {
    setEditing(null);
    setDraft("");
  };

  const commit = async (cluster: number) => {
    const name = draft.trim();
    if (name.length === 0) {
      cancel();
      return;
    }
    setSaving(true);
    try {
      const labels = await renameSessionSpeaker(sessionDir, cluster, name);
      onRenamed(labels);
      toast.success(`Renamed to ${name}`, {
        description: labelsByCluster.get(cluster)?.has_embedding
          ? "Saved. Future recordings will recognise this voice."
          : "Saved for this recording (too little audio to remember).",
      });
      cancel();
    } catch (e) {
      console.error("rename_session_speaker:", e);
      toast.error("Could not rename speaker", { description: String(e) });
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      <span className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        Speakers
      </span>
      {speakers.map((s) => {
        const meta = labelsByCluster.get(s.cluster);
        const remembered = meta?.has_embedding ?? false;
        const auto = meta?.auto_named ?? false;
        const suggested = meta?.suggested_name ?? null;

        // Medium-confidence match → "· <name>?" with confirm / reject.
        if (suggested && editing !== s.cluster) {
          const busy = acting === s.cluster;
          return (
            <span
              key={s.cluster}
              className="inline-flex items-center gap-1 rounded-full border border-input bg-card px-1.5 py-0.5 text-2xs"
            >
              <span
                className={cn(
                  "inline-flex items-center rounded-full px-1.5 py-0.5 font-medium",
                  s.pillClass
                )}
              >
                {s.label}
              </span>
              <span className="text-muted-foreground">
                · {suggested}?
              </span>
              <button
                type="button"
                onClick={() => void respond(s.cluster, true)}
                disabled={busy}
                aria-label={`Confirm this is ${suggested}`}
                title={`Yes, this is ${suggested}`}
                className="inline-flex h-4 w-4 items-center justify-center rounded text-emerald-600 hover:bg-emerald-500/10 dark:text-emerald-400"
              >
                {busy ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <Check className="h-3 w-3" />
                )}
              </button>
              <button
                type="button"
                onClick={() => void respond(s.cluster, false)}
                disabled={busy}
                aria-label={`This is not ${suggested}`}
                title={`No, not ${suggested}`}
                className="inline-flex h-4 w-4 items-center justify-center rounded text-muted-foreground hover:bg-muted"
              >
                <X className="h-3 w-3" />
              </button>
              <button
                type="button"
                onClick={() => begin(s)}
                disabled={busy}
                aria-label="Rename instead"
                title="Rename instead"
                className="inline-flex h-4 w-4 items-center justify-center rounded text-muted-foreground hover:bg-muted"
              >
                <Pencil className="h-2.5 w-2.5" />
              </button>
            </span>
          );
        }

        if (editing === s.cluster) {
          return (
            <span
              key={s.cluster}
              className="inline-flex items-center gap-1 rounded-full border border-input bg-card px-1.5 py-0.5"
            >
              <input
                // Focus belongs here: the input only mounts in direct
                // response to the user clicking "edit" on this chip.
                // eslint-disable-next-line jsx-a11y/no-autofocus
                autoFocus
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void commit(s.cluster);
                  if (e.key === "Escape") cancel();
                }}
                placeholder={`Speaker ${s.number}`}
                aria-label={`Name for Speaker ${s.number}`}
                disabled={saving}
                className="h-5 w-28 bg-transparent text-2xs outline-none placeholder:text-muted-foreground"
              />
              <button
                type="button"
                onClick={() => void commit(s.cluster)}
                disabled={saving}
                aria-label="Save name"
                className="inline-flex h-4 w-4 items-center justify-center rounded text-emerald-600 hover:bg-emerald-500/10 dark:text-emerald-400"
              >
                {saving ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <Check className="h-3 w-3" />
                )}
              </button>
              <button
                type="button"
                onClick={cancel}
                disabled={saving}
                aria-label="Cancel"
                className="inline-flex h-4 w-4 items-center justify-center rounded text-muted-foreground hover:bg-muted"
              >
                <X className="h-3 w-3" />
              </button>
            </span>
          );
        }

        return (
          <button
            key={s.cluster}
            type="button"
            onClick={() => begin(s)}
            title={
              remembered
                ? "Rename — this voice will be remembered across recordings"
                : "Rename for this recording (too little audio to remember)"
            }
            className={cn(
              "group inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-2xs font-medium transition-opacity hover:opacity-80",
              s.pillClass
            )}
          >
            {s.label}
            {auto && (
              <span className="text-[9px] font-normal opacity-70">auto</span>
            )}
            <Pencil className="h-2.5 w-2.5 opacity-50 group-hover:opacity-100" />
          </button>
        );
      })}
    </div>
  );
}
