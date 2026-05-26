import * as React from "react";
import {
  AudioLines,
  Brain,
  CheckCircle2,
  Cloud,
  ExternalLink,
  Mic,
  Monitor,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { Badge } from "@/shared/ui/badge";
import { cn } from "@/shared/lib/utils";
import {
  listPermissions,
  openPermissionSettings,
  setProviderKey,
} from "@/shared/lib/ipc";
import { useSettingsStore } from "@/shared/stores/settings-store";
import type { PermissionRow } from "@/shared/types/PermissionRow";

type Transcriber = "local_whisper" | "openai";

/**
 * One-screen first-run conductor. v2 finding 001 / GET-24 — Hero.
 *
 * Renders while `Settings.onboarding_completed` is false. Three
 * actions stacked vertically:
 *
 *   1. Grant the two required TCC permissions (Microphone + Screen
 *      Recording) via the System Settings deep links from #003.
 *   2. Pick the ASR: Local Whisper (default, free) or OpenAI Whisper
 *      API (paid, faster) — with the key-paste field when OpenAI is
 *      selected.
 *   3. A primed Record button labelled "I'm ready" that flips
 *      onboarding_completed and dismisses the conductor.
 *
 * Bias toward zero friction: the user can finish without granting
 * permissions or pasting a key — the rows just keep showing yellow
 * dots and the Record button stays primed. The downstream record
 * flow surfaces missing permissions per-attempt.
 */
export function FirstRunConductor({ onFinish }: { onFinish: () => void }) {
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.save);
  const [rows, setRows] = React.useState<PermissionRow[]>([]);
  const [transcriber, setTranscriber] = React.useState<Transcriber>(
    (settings?.transcriber as Transcriber) ?? "local_whisper"
  );
  const [openaiKey, setOpenaiKey] = React.useState(settings?.openai_api_key ?? "");
  const [savingKey, setSavingKey] = React.useState(false);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const next = await listPermissions();
        if (!cancelled) setRows(next);
      } catch (e) {
        if (!cancelled) console.error("list_permissions:", e);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const requiredPermissions = React.useMemo(
    () => rows.filter((r) => r.permission === "microphone" || r.permission === "screen_recording"),
    [rows]
  );

  const finish = React.useCallback(async () => {
    if (transcriber === "openai" && openaiKey.trim().length > 0) {
      try {
        setSavingKey(true);
        await setProviderKey("openai", openaiKey.trim());
      } catch (e) {
        console.error("set_provider_key:", e);
        toast.error("Could not save OpenAI key", { description: String(e) });
        setSavingKey(false);
        return;
      }
      setSavingKey(false);
    }
    if (!settings) return;
    try {
      await saveSettings({
        ...settings,
        transcriber,
        onboarding_completed: true,
      });
      toast.success("You're set up", {
        description: "Press Cmd-R any time to start recording.",
      });
      onFinish();
    } catch (e) {
      console.error("update settings on first-run finish:", e);
      toast.error("Could not save preferences", { description: String(e) });
    }
  }, [openaiKey, settings, transcriber, saveSettings, onFinish]);

  if (!settings) return null;

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-8 py-12">
      <header data-drag="" className="select-none">
        <div className="flex items-center gap-3">
          <Sparkles className="h-6 w-6 text-primary" />
          <h1 className="font-serif text-4xl font-medium tracking-tight">Welcome to Attune</h1>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          A local-first meeting recorder. Two permissions, one transcription choice, and
          you&apos;re ready to capture your first meeting.
        </p>
      </header>

      <Card>
        <CardContent className="flex flex-col gap-4 py-5">
          <div className="flex items-center gap-2">
            <ShieldCheck className="h-4 w-4 text-muted-foreground" />
            <h2 className="font-medium">Grant permissions</h2>
            <Badge variant="outline" className="text-2xs">
              required
            </Badge>
          </div>
          <ul className="flex flex-col gap-2">
            {requiredPermissions.map((row) => (
              <li
                key={row.permission}
                className="flex items-start gap-3 rounded-md border border-border bg-muted/30 p-3"
              >
                {row.permission === "microphone" ? (
                  <Mic className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
                ) : (
                  <Monitor className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
                )}
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium">
                    {row.permission === "microphone" ? "Microphone" : "Screen Recording"}
                  </p>
                  <p className="mt-0.5 text-xs text-muted-foreground">{row.rationale}</p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    openPermissionSettings(row.permission).catch((e) =>
                      console.error("open_permission_settings:", e)
                    );
                  }}
                  className="shrink-0 gap-1"
                >
                  <ExternalLink className="h-3 w-3" />
                  Open
                </Button>
              </li>
            ))}
            {requiredPermissions.length === 0 ? (
              <li className="text-xs italic text-muted-foreground">Loading permissions…</li>
            ) : null}
          </ul>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="flex flex-col gap-4 py-5">
          <div className="flex items-center gap-2">
            <Brain className="h-4 w-4 text-muted-foreground" />
            <h2 className="font-medium">Pick transcription</h2>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <TranscriberChoice
              selected={transcriber === "local_whisper"}
              onClick={() => setTranscriber("local_whisper")}
              icon={ShieldCheck}
              title="Local Whisper"
              detail="Runs on your Mac. Free. No network. Slower on first run while the model downloads."
            />
            <TranscriberChoice
              selected={transcriber === "openai"}
              onClick={() => setTranscriber("openai")}
              icon={Cloud}
              title="OpenAI Whisper"
              detail="Cloud API. Faster on long meetings. Needs your OpenAI key."
            />
          </div>
          {transcriber === "openai" ? (
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="text-xs text-muted-foreground">
                OpenAI API key (stored in macOS Keychain, never on disk in plain text)
              </span>
              <input
                type="password"
                value={openaiKey}
                onChange={(e) => setOpenaiKey(e.target.value)}
                placeholder="sk-..."
                autoComplete="off"
                spellCheck={false}
                className="rounded-md border border-border bg-background px-3 py-1.5 font-mono text-xs outline-none focus:border-ring"
              />
            </label>
          ) : null}
        </CardContent>
      </Card>

      <div
        className={cn(
          "flex items-center justify-between rounded-lg border border-primary/30 bg-primary/5 p-4"
        )}
      >
        <div className="flex items-center gap-2">
          <CheckCircle2 className="h-4 w-4 text-primary" />
          <p className="text-sm">You can change everything later in Preferences (Cmd-,).</p>
        </div>
        <Button onClick={finish} disabled={savingKey} className="gap-2">
          <AudioLines className="h-4 w-4" />
          I&apos;m ready
        </Button>
      </div>
    </div>
  );
}

interface ChoiceProps {
  selected: boolean;
  onClick: () => void;
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  detail: string;
}

function TranscriberChoice({ selected, onClick, icon: Icon, title, detail }: ChoiceProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={selected}
      className={cn(
        "flex flex-col items-start gap-2 rounded-md border p-3 text-left transition-colors",
        selected
          ? "border-primary bg-primary/5 ring-1 ring-primary/30"
          : "border-border bg-card hover:bg-muted/40"
      )}
    >
      <div className="flex items-center gap-2">
        <Icon className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium">{title}</span>
        {selected ? <CheckCircle2 className="h-3.5 w-3.5 text-primary" /> : null}
      </div>
      <p className="text-xs text-muted-foreground">{detail}</p>
    </button>
  );
}
