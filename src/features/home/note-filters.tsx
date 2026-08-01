import { Search } from "lucide-react";

import { Input } from "@/shared/ui/input";
import { cn } from "@/shared/lib/utils";

export type TranscriptFilter = "all" | "transcribed" | "untranscribed";
export type SortOrder = "newest" | "oldest";

interface Props {
  query: string;
  onQueryChange: (next: string) => void;
  filter: TranscriptFilter;
  onFilterChange: (next: TranscriptFilter) => void;
  sort: SortOrder;
  onSortChange: (next: SortOrder) => void;
}

const FILTERS: { id: TranscriptFilter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "transcribed", label: "Transcribed" },
  { id: "untranscribed", label: "Untranscribed" },
];

export function NoteFilters({
  query,
  onQueryChange,
  filter,
  onFilterChange,
  sort,
  onSortChange,
}: Props) {
  return (
    <div className="flex flex-wrap items-center gap-3">
      <div className="relative min-w-[220px] flex-1">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
        <Input
          value={query}
          onChange={(e) => onQueryChange(e.target.value)}
          placeholder="Search by date or label…"
          className="pl-9"
          aria-label="Search recordings"
        />
      </div>

      <div
        role="radiogroup"
        aria-label="Filter by transcript status"
        className="inline-flex items-center gap-1 rounded-md border border-border bg-card p-1"
      >
        {FILTERS.map((f) => {
          const active = filter === f.id;
          return (
            <button
              type="button"
              key={f.id}
              role="radio"
              aria-checked={active}
              onClick={() => onFilterChange(f.id)}
              className={cn(
                "rounded px-2.5 py-1 text-xs font-medium transition-colors",
                active
                  ? "bg-accent text-foreground"
                  : "text-muted-foreground hover:bg-secondary"
              )}
            >
              {f.label}
            </button>
          );
        })}
      </div>

      <label className="inline-flex items-center gap-2 text-xs text-muted-foreground">
        Sort
        <select
          value={sort}
          onChange={(e) => onSortChange(e.target.value as SortOrder)}
          className="h-8 rounded-md border border-input bg-card px-2 text-xs focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          aria-label="Sort order"
        >
          <option value="newest">Newest first</option>
          <option value="oldest">Oldest first</option>
        </select>
      </label>
    </div>
  );
}
