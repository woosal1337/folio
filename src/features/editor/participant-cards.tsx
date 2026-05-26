/**
 * Per-participant context cards rendered at the top of the Editor
 * page. Heuristically matches memories whose `kind == 'person'`
 * against the transcript text and renders a small chip per
 * recognised participant with their role + last-interaction note.
 *
 * The match is loose on purpose (substring of the key tail after
 * the dot — e.g. `person.alice` → `alice`) so the model's
 * dotted-handle style works without us forcing the user to also
 * record full names. A future iteration may use proper NER, but
 * the substring heuristic catches the common case at zero cost.
 *
 * v2 roadmap finding 033.
 */

import * as React from "react";
import { Users } from "lucide-react";

import { MetaList, MetaRow } from "@/shared/ui/meta-list";
import { listMemories } from "@/shared/lib/ipc";
import type { Memory } from "@/shared/types/Memory";
import type { SessionTranscript } from "@/shared/types/SessionTranscript";

interface Props {
  transcript: SessionTranscript;
}

export function ParticipantCards({ transcript }: Props) {
  const [matches, setMatches] = React.useState<Memory[]>([]);

  // Flatten the transcript once per render of this prop; cheap.
  const text = React.useMemo(() => {
    return transcript.channels
      .flatMap((c) => c.segments.map((s) => s.text))
      .join(" ")
      .toLowerCase();
  }, [transcript]);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const all = await listMemories({
          query: null,
          kinds: ["person"],
          include_archived: false,
          limit: null,
        });
        if (cancelled) return;
        const hits = all.filter((m) => {
          const tail = (m.key ?? "").split(".").pop()?.toLowerCase();
          if (!tail || tail.length < 3) return false;
          return text.includes(tail);
        });
        setMatches(hits);
      } catch (e) {
        if (!cancelled) {
          console.error("ParticipantCards: listMemories failed", e);
          setMatches([]);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [text]);

  if (matches.length === 0) return null;

  return (
    <section aria-label="Participants mentioned in this meeting">
      <header className="mb-2 flex items-center gap-2 text-xs uppercase tracking-wider text-muted-foreground">
        <Users className="h-3.5 w-3.5" />
        Participants ({matches.length})
      </header>
      <MetaList>
        {matches.map((m) => (
          <MetaRow
            key={m.id}
            label={<span className="font-medium text-foreground">{labelFor(m)}</span>}
            value={m.content}
            mono={false}
            hint={
              m.source_session_label ? `last: ${m.source_session_label}` : undefined
            }
          />
        ))}
      </MetaList>
    </section>
  );
}

/** Pretty-print a person memory's key as a Title-Case name. */
function labelFor(m: Memory): string {
  const tail = (m.key ?? "").split(".").pop() ?? "";
  return tail
    .split(/[-_]/)
    .map((p) => (p.length === 0 ? p : p[0].toUpperCase() + p.slice(1)))
    .join(" ");
}
