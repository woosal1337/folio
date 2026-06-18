import { cn } from "@/lib/utils";

const pattern = [
  0.3, 0.55, 0.85, 0.45, 0.7, 1, 0.6, 0.35, 0.8, 0.5, 0.9, 0.4, 0.65, 0.95, 0.5, 0.75,
  0.35, 0.6, 0.85, 0.45,
];

export function WaveBars({
  className,
  bars = pattern,
  animated = true,
}: {
  className?: string;
  bars?: number[];
  animated?: boolean;
}) {
  return (
    <div
      className={cn("flex h-8 w-full items-center justify-center gap-[2px]", className)}
      aria-hidden
    >
      {bars.map((height, index) => (
        <span
          key={index}
          className={cn(
            "w-px flex-1 origin-center rounded-full bg-current",
            animated && "motion-safe:animate-[wave-rise_1.4s_ease-in-out_infinite]"
          )}
          style={{
            height: `${Math.round(height * 100)}%`,
            animationDelay: `${(index % 8) * 0.11}s`,
          }}
        />
      ))}
    </div>
  );
}
