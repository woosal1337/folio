import * as React from "react";
import {
  AudioLines,
  Brain,
  CheckCircle2,
  Cloud,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { cn } from "@/shared/lib/utils";
import { humanizeError } from "@/shared/lib/errors";
import { setProviderKey } from "@/shared/lib/ipc";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { PermissionsScreen } from "./permissions-screen";

type Transcriber = "local_whisper" | "openai";

type Step = "permissions" | "transcriber";

export function FirstRunConductor({ onFinish }: { onFinish: () => void }) {
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.save);

  const [step, setStep] = React.useState<Step>("permissions");
  const [transcriber, setTranscriber] = React.useState<Transcriber>(
    (settings?.transcriber as Transcriber) ?? "local_whisper"
  );
  const [openaiKey, setOpenaiKey] = React.useState("");
  const [savingKey, setSavingKey] = React.useState(false);

  const finish = React.useCallback(async () => {
    if (transcriber === "openai" && openaiKey.trim().length > 0) {
      try {
        setSavingKey(true);
        await setProviderKey("openai", openaiKey.trim());
      } catch (e) {
        console.error("set_provider_key:", e);
        toast.error("Could not save OpenAI key", { description: humanizeError(e) });
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
      toast.error("Could not save preferences", { description: humanizeError(e) });
    }
  }, [openaiKey, settings, transcriber, saveSettings, onFinish]);

  if (!settings) return null;

  if (step === "permissions") {
    return <PermissionsScreen onContinue={() => setStep("transcriber")} />;
  }

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-8 py-12">
      <header data-drag="" className="select-none">
        <div className="flex items-center gap-3">
          <Sparkles className="h-6 w-6 text-primary" />
          <h1 className="font-serif text-4xl font-medium tracking-tight">
            Welcome to Folio
          </h1>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          One last thing — pick how you want transcripts to happen.
        </p>
      </header>

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
          <p className="text-sm">
            You can change everything later in Preferences (Cmd-,).
          </p>
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

function TranscriberChoice({
  selected,
  onClick,
  icon: Icon,
  title,
  detail,
}: ChoiceProps) {
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
