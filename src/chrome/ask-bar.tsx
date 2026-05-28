import * as React from "react";
import { useNavigate } from "react-router-dom";
import { ListTodo, Sparkles } from "lucide-react";

/**
 * GET-156 — the omnipresent "Ask anything" bar.
 *
 * A slim bottom bar (used on Home) that opens the full cross-library
 * Chat seeded with the typed question. Replaces the Chat sidebar tab as
 * the primary way into Ask. Inside a note, the note's own dock carries a
 * scoped Ask instead.
 */
export function AskBar() {
  const navigate = useNavigate();
  const [value, setValue] = React.useState("");

  const open = React.useCallback(
    (seed?: string) => {
      navigate("/chat", seed ? { state: { seed } } : undefined);
    },
    [navigate]
  );

  return (
    <div className="sticky bottom-0 z-10 -mx-8 border-t border-border bg-background/80 px-8 py-3 backdrop-blur">
      <div className="mx-auto flex max-w-3xl items-center gap-2">
        <form
          className="flex flex-1 items-center gap-2 rounded-full border border-input bg-card px-4 py-2 shadow-sm focus-within:ring-2 focus-within:ring-ring"
          onSubmit={(e) => {
            e.preventDefault();
            const q = value.trim();
            if (q) open(q);
            setValue("");
          }}
        >
          <Sparkles className="h-4 w-4 shrink-0 text-muted-foreground" />
          <input
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="Ask anything across your notes…"
            aria-label="Ask anything across your notes"
            className="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
        </form>
        <button
          type="button"
          onClick={() =>
            open(
              "List my open action items across recent meetings, grouped by meeting."
            )
          }
          className="inline-flex shrink-0 items-center gap-1.5 rounded-full border border-border bg-card px-3 py-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
        >
          <ListTodo className="h-3.5 w-3.5" />
          List recent todos
        </button>
      </div>
    </div>
  );
}
