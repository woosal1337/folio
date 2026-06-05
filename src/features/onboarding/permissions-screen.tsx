import * as React from "react";
import { Check, Loader2, Mic, Monitor, ShieldCheck } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";
import { humanizeError } from "@/shared/lib/errors";
import { listPermissions, requestPermission } from "@/shared/lib/ipc";
import type { PermissionRow } from "@/shared/types/PermissionRow";

type Slot = "microphone" | "screen_recording";

interface SlotConfig {
  slot: Slot;
  title: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
}

const SLOTS: SlotConfig[] = [
  {
    slot: "microphone",
    title: "My microphone",
    description: "Capture what you say so Folio can transcribe it.",
    icon: Mic,
  },
  {
    slot: "screen_recording",
    title: "The meeting audio",
    description:
      "Capture what the other participants say — through your computer's audio.",
    icon: Monitor,
  },
];

interface Props {
  onContinue: () => void;

  onSkip?: () => void;
}

export function PermissionsScreen({ onContinue, onSkip }: Props) {
  const [rows, setRows] = React.useState<PermissionRow[]>([]);
  const [pending, setPending] = React.useState<Set<Slot>>(new Set());

  const refresh = React.useCallback(async () => {
    try {
      const next = await listPermissions();
      setRows(next);
    } catch (e) {
      console.error("list_permissions:", e);
    }
  }, []);

  React.useEffect(() => {
    void refresh();
    const onFocus = () => void refresh();
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, [refresh]);

  const byPermission = React.useMemo(() => {
    const map = new Map<string, PermissionRow>();
    for (const r of rows) map.set(r.permission, r);
    return map;
  }, [rows]);

  const allGranted = SLOTS.every((s) => byPermission.get(s.slot)?.status === "granted");

  const handleEnable = async (slot: Slot) => {
    setPending((s) => new Set(s).add(slot));
    try {
      await requestPermission(slot);
    } catch (e) {
      console.error("request_permission:", e);
      toast.error("Could not request permission", { description: humanizeError(e) });
    } finally {
      window.setTimeout(() => {
        setPending((s) => {
          const next = new Set(s);
          next.delete(slot);
          return next;
        });
        void refresh();
      }, 1200);
    }
  };

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-7 px-8 py-12">
      <header className="select-none" data-drag="">
        <div className="flex items-center gap-3">
          <ShieldCheck className="h-6 w-6 text-primary" />
          <p className="font-mono text-xs uppercase tracking-wider text-muted-foreground">
            Permissions
          </p>
        </div>
        <h1 className="mt-3 font-serif text-3xl font-medium tracking-tight">
          Allow Folio to transcribe your meetings
        </h1>
        <p className="mt-3 max-w-prose text-sm text-muted-foreground">
          Folio captures your meeting audio locally. No bots. No upload.
        </p>
      </header>

      <div
        role="group"
        aria-label="Required permissions"
        className="overflow-hidden rounded-xl border border-border bg-card"
      >
        {SLOTS.map((cfg, idx) => {
          const row = byPermission.get(cfg.slot);
          const granted = row?.status === "granted";
          const isPending = pending.has(cfg.slot);
          const Icon = cfg.icon;
          return (
            <div
              key={cfg.slot}
              className={cn(
                "flex items-start gap-4 px-5 py-4",
                idx > 0 && "border-t border-border"
              )}
            >
              <Icon className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
              <div className="flex-1">
                <p className="text-sm font-medium text-foreground">{cfg.title}</p>
                <p className="mt-0.5 text-xs text-muted-foreground">
                  {cfg.description}
                </p>
              </div>
              <div className="shrink-0">
                {granted ? (
                  <div
                    role="status"
                    aria-live="polite"
                    aria-label={`${cfg.title}: granted`}
                    className="flex items-center gap-1.5"
                  >
                    <Badge
                      variant="outline"
                      className="gap-1.5 border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300"
                    >
                      <Check className="h-3 w-3" />
                      Granted
                    </Badge>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 text-2xs text-muted-foreground"
                      onClick={() => handleEnable(cfg.slot)}
                      title="Re-open System Settings for this permission"
                      aria-label={`Re-prompt ${cfg.title}`}
                    >
                      Re-prompt
                    </Button>
                  </div>
                ) : (
                  <Button
                    size="sm"
                    onClick={() => handleEnable(cfg.slot)}
                    disabled={isPending}
                    aria-label={`Enable ${cfg.title}`}
                    className="gap-1.5"
                  >
                    {isPending ? (
                      <>
                        <Loader2 className="h-3.5 w-3.5 animate-spin" />
                        Granting…
                      </>
                    ) : (
                      <>Enable</>
                    )}
                  </Button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <span>You can change this later in System Settings.</span>
        <div className="flex items-center gap-2">
          {onSkip ? (
            <Button variant="ghost" size="sm" onClick={onSkip}>
              Skip for now
            </Button>
          ) : null}
          <Button
            size="sm"
            onClick={onContinue}
            disabled={!allGranted}
            aria-label="Continue"
          >
            Continue
          </Button>
        </div>
      </div>
    </div>
  );
}
