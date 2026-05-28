import { Badge } from "@/shared/ui/badge";
import { cn } from "@/shared/lib/utils";

interface Props {
  recording: boolean;
  label: string;
  /** GET-149: capture is paused — a note is open but not recording. */
  paused?: boolean;
}

/**
 * Compact status indicator: pulsing red dot + "recording" / "paused" /
 * "idle" + elapsed timer. Lives in its own file so the Library page and
 * the Record page can both render it.
 */
export function StatusPill({ recording, label, paused = false }: Props) {
  const state = recording ? "recording" : paused ? "paused" : "idle";
  return (
    <Badge
      variant="outline"
      className="gap-2 px-3 py-1 font-mono tabular-nums tracking-tight"
    >
      <span
        className={cn(
          "inline-block h-2 w-2 rounded-full",
          recording && "animate-pulse-record bg-destructive",
          paused && !recording && "bg-amber-500",
          !recording && !paused && "border border-muted-foreground"
        )}
      />
      <span>{state}</span>
      <span aria-hidden="true">·</span>
      <span>{label}</span>
    </Badge>
  );
}
