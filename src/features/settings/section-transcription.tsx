import { KeyRound } from "lucide-react";

import { Badge } from "@/shared/ui/badge";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { cn } from "@/shared/lib/utils";
import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

const PROVIDERS: { id: string; label: string; desc: string }[] = [
  {
    id: "openai",
    label: "OpenAI Whisper API",
    desc: "Uploaded to OpenAI · ~$0.006/min · multilingual",
  },
  {
    id: "local_whisper",
    label: "Local Whisper",
    desc: "Runs on this Mac · lands in a future session",
  },
];

const LANGUAGES: { value: string; label: string }[] = [
  { value: "auto", label: "Auto-detect" },
  { value: "en", label: "English" },
  { value: "tr", label: "Turkish" },
  { value: "az", label: "Azerbaijani" },
  { value: "ru", label: "Russian" },
  { value: "de", label: "German" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
  { value: "ar", label: "Arabic" },
  { value: "ja", label: "Japanese" },
  { value: "zh", label: "Chinese" },
];

export function SectionTranscription({ settings, onChange }: Props) {
  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Transcription</h2>

      <section className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Provider
        </Label>
        <div className="grid gap-2">
          {PROVIDERS.map((p) => {
            const selected = settings.transcriber === p.id;
            return (
              <button
                type="button"
                key={p.id}
                onClick={() => onChange("transcriber", p.id)}
                className={cn(
                  "flex flex-col items-start gap-1 rounded-lg border p-4 text-left transition-colors",
                  selected
                    ? "border-primary bg-accent"
                    : "border-border bg-card hover:bg-secondary"
                )}
              >
                <div className="flex w-full items-center justify-between">
                  <span className="font-medium">{p.label}</span>
                  {selected && (
                    <Badge variant="accent" className="text-2xs">
                      selected
                    </Badge>
                  )}
                </div>
                <span className="text-xs text-muted-foreground">{p.desc}</span>
              </button>
            );
          })}
        </div>
      </section>

      {settings.transcriber === "openai" && (
        <section className="space-y-3">
          <Label
            htmlFor="openai-key"
            className="flex items-center gap-2 text-xs font-medium uppercase tracking-wider text-muted-foreground"
          >
            <KeyRound className="h-3.5 w-3.5" />
            OpenAI API key
          </Label>
          <Input
            id="openai-key"
            type="password"
            placeholder="sk-..."
            value={settings.openai_api_key}
            onChange={(e) => onChange("openai_api_key", e.target.value)}
            className="font-mono"
          />
          <p className="text-xs text-muted-foreground">
            Stored locally. Sent only to api.openai.com.
          </p>
        </section>
      )}

      <section className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Language
        </Label>
        <select
          value={settings.transcription_language}
          onChange={(e) => onChange("transcription_language", e.target.value)}
          className="h-9 w-full rounded-md border border-input bg-card px-3 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          {LANGUAGES.map((l) => (
            <option key={l.value} value={l.value}>
              {l.label}
            </option>
          ))}
        </select>
        <p className="text-xs text-muted-foreground">
          Set a language if you record predominantly in one. Auto detects per segment.
        </p>
      </section>
    </div>
  );
}
