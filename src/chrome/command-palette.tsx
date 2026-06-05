import * as React from "react";
import { Search } from "lucide-react";

import { Dialog, DialogContent } from "@/shared/ui/dialog";
import { cn } from "@/shared/lib/utils";
import {
  rank,
  type CommandItem,
  type CommandSource,
} from "@/shared/lib/command-palette";

interface Props {
  open: boolean;
  onClose: () => void;
  sources: CommandSource[];
}

export function CommandPalette({ open, onClose, sources }: Props) {
  const [items, setItems] = React.useState<CommandItem[]>([]);
  const [dynamicItems, setDynamicItems] = React.useState<CommandItem[]>([]);
  const [query, setQuery] = React.useState("");
  const [activeIndex, setActiveIndex] = React.useState(0);
  const inputRef = React.useRef<HTMLInputElement>(null);

  React.useEffect(() => {
    if (!open) {
      setItems([]);
      setDynamicItems([]);
      setQuery("");
      setActiveIndex(0);
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const batches = await Promise.all(sources.map((s) => s.load()));
        if (!cancelled) setItems(batches.flat());
      } catch (e) {
        if (!cancelled) console.error("command-palette source load:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [open, sources]);

  React.useEffect(() => {
    const searchers = sources
      .map((s) => s.search)
      .filter((fn): fn is NonNullable<typeof fn> => fn !== undefined);
    const needle = query.trim();
    if (!open || searchers.length === 0 || needle.length < 2) {
      setDynamicItems([]);
      return;
    }
    let cancelled = false;
    const t = window.setTimeout(() => {
      Promise.all(searchers.map((fn) => fn(needle)))
        .then((batches) => {
          if (!cancelled) setDynamicItems(batches.flat());
        })
        .catch((e) => console.error("command-palette source search:", e));
    }, 150);
    return () => {
      cancelled = true;
      window.clearTimeout(t);
    };
  }, [open, query, sources]);

  const ranked = React.useMemo(() => {
    const seen = new Set<string>();
    const merged = [...items, ...dynamicItems].filter((i) => {
      if (seen.has(i.id)) return false;
      seen.add(i.id);
      return true;
    });
    return rank(merged, query).slice(0, 50);
  }, [items, dynamicItems, query]);

  React.useEffect(() => {
    setActiveIndex(0);
  }, [query, items.length]);

  React.useEffect(() => {
    if (open) inputRef.current?.focus();
  }, [open]);

  const onKey = React.useCallback(
    (event: React.KeyboardEvent<HTMLInputElement>) => {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setActiveIndex((i) => Math.min(i + 1, Math.max(0, ranked.length - 1)));
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        setActiveIndex((i) => Math.max(0, i - 1));
        return;
      }
      if (event.key === "Enter") {
        event.preventDefault();
        const hit = ranked[activeIndex];
        if (hit) {
          void hit.action();
          onClose();
        }
      }
    },
    [ranked, activeIndex, onClose]
  );

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-2xl gap-0 p-0">
        <div className="flex items-center gap-2 border-b border-border px-4 py-3">
          <Search className="h-4 w-4 text-muted-foreground" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKey}
            placeholder="Search recordings, tasks, memories, or jump to anything…"
            className="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          />
          <span className="font-mono text-2xs text-muted-foreground">esc</span>
        </div>
        <ul role="listbox" className="max-h-[60vh] overflow-y-auto py-1">
          {ranked.length === 0 ? (
            <li className="px-4 py-6 text-center text-xs text-muted-foreground">
              {query.trim().length > 0 ? "No matches." : "Type to search…"}
            </li>
          ) : (
            ranked.map((item, idx) => (
              <li
                key={item.id}
                role="option"
                aria-selected={idx === activeIndex}
                tabIndex={-1}
                onMouseEnter={() => setActiveIndex(idx)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    void item.action();
                    onClose();
                  }
                }}
                onClick={() => {
                  void item.action();
                  onClose();
                }}
                className={cn(
                  "flex cursor-pointer items-center gap-3 px-4 py-2 text-sm",
                  idx === activeIndex && "bg-accent text-accent-foreground"
                )}
              >
                <KindGlyph kind={item.kind} />
                <div className="min-w-0 flex-1">
                  <p className="truncate">{item.title}</p>
                  {item.subtitle ? (
                    <p className="truncate text-2xs text-muted-foreground">
                      {item.subtitle}
                    </p>
                  ) : null}
                </div>
                {item.shortcut ? (
                  <span className="font-mono text-2xs text-muted-foreground">
                    {item.shortcut}
                  </span>
                ) : null}
              </li>
            ))
          )}
        </ul>
      </DialogContent>
    </Dialog>
  );
}

function KindGlyph({ kind }: { kind: CommandItem["kind"] }) {
  const label = (
    {
      recording: "REC",
      task: "TASK",
      memory: "MEM",
      "agent-run": "AI",
      decision: "DEC",
      verb: "GO",
    } as const
  )[kind];
  return (
    <span className="rounded border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] uppercase tracking-wider text-muted-foreground">
      {label}
    </span>
  );
}
