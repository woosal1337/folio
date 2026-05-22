import { Badge } from "@/shared/ui/badge";
import { cn } from "@/shared/lib/utils";

interface Props {
  recording: boolean;
  label: string;
}

/**
 * Compact status indicator: pulsing red dot + "recording" / "idle" +
 * elapsed timer. Lives in its own file so the Library page and the
 * Record page can both render it.
 */
export function StatusPill({ recording, label }: Props) {
  return (
    <Badge variant="outline" className="gap-2 px-3 py-1 font-mono tracking-tight">
      <span
        className={cn(
          "inline-block h-2 w-2 rounded-full",
          recording
            ? "animate-pulse-record bg-destructive"
            : "border border-muted-foreground"
        )}
      />
      <span>{recording ? "recording" : "idle"}</span>
      <span>·</span>
      <span>{label}</span>
    </Badge>
  );
}
