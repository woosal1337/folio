import { cn } from "@/shared/lib/utils";
import type { Theme } from "@/shared/hooks/use-theme";

interface Props {
  theme: Theme;
  onChange: (t: Theme) => void;
}

export function SectionAppearance({ theme, onChange }: Props) {
  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Appearance</h2>
      <p className="text-sm text-muted-foreground">
        Light is the default. Dark is available for late-night work.
      </p>
      <div className="grid grid-cols-2 gap-3">
        {(["light", "dark"] as Theme[]).map((t) => {
          const selected = theme === t;
          return (
            <button
              type="button"
              key={t}
              onClick={() => onChange(t)}
              className={cn(
                "flex flex-col items-start gap-2 rounded-lg border p-4 text-left transition-colors",
                selected
                  ? "border-primary bg-accent"
                  : "border-border bg-card hover:bg-secondary"
              )}
            >
              <div
                className={cn(
                  "h-16 w-full rounded-md border",
                  t === "light"
                    ? "border-zinc-200 bg-[#F5F2EC]"
                    : "border-zinc-700 bg-[#0d0d10]"
                )}
              />
              <span className="font-medium capitalize">{t}</span>
            </button>
          );
        })}
      </div>
    </div>
  );
}
