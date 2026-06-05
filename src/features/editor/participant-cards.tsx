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

function labelFor(m: Memory): string {
  const tail = (m.key ?? "").split(".").pop() ?? "";
  return tail
    .split(/[-_]/)
    .map((p) => {
      if (p.length === 0) return p;
      const first = p[0] ?? "";
      return first.toUpperCase() + p.slice(1);
    })
    .join(" ");
}
