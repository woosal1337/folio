import * as React from "react";
import { CheckCircle2, KeyRound, Loader2, Sparkles, XCircle, Zap } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import { humanizeError } from "@/shared/lib/errors";
import { cn } from "@/shared/lib/utils";
import {
  deleteProviderKey,
  listProviders,
  setProviderKey,
  testProvider,
} from "@/shared/lib/ipc";
import type { ProviderId } from "@/shared/types/ProviderId";
import type { ProviderStatus } from "@/shared/types/ProviderStatus";
import type { Settings } from "@/shared/types/Settings";

interface SectionAiProps {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

type ProviderRowState = {
  pendingKey: string;
  saving: boolean;
  testing: boolean;
  testResult: "ok" | "fail" | null;
  testError: string | null;
};

const INITIAL_ROW_STATE: ProviderRowState = {
  pendingKey: "",
  saving: false,
  testing: false,
  testResult: null,
  testError: null,
};

export function SectionAi({ settings, onChange }: SectionAiProps) {
  const [providers, setProviders] = React.useState<ProviderStatus[] | null>(null);
  const [rows, setRows] = React.useState<Record<string, ProviderRowState>>({});

  const refresh = React.useCallback(async () => {
    try {
      const list = await listProviders();
      setProviders(list);
    } catch (e) {
      console.error("listProviders:", e);
      toast.error("Could not load AI providers", { description: humanizeError(e) });
    }
  }, []);

  React.useEffect(() => {
    void refresh();
  }, [refresh]);

  const rowState = (id: string): ProviderRowState => rows[id] ?? INITIAL_ROW_STATE;

  const updateRow = (id: string, patch: Partial<ProviderRowState>) => {
    setRows((prev) => ({
      ...prev,
      [id]: { ...(prev[id] ?? INITIAL_ROW_STATE), ...patch },
    }));
  };

  const onSaveKey = async (provider: ProviderId) => {
    const state = rowState(provider);
    const key = state.pendingKey.trim();
    if (!key) {
      toast.error("Paste a key first");
      return;
    }
    updateRow(provider, { saving: true, testResult: null, testError: null });
    try {
      await setProviderKey(provider, key);
      updateRow(provider, { saving: false, pendingKey: "" });
      await refresh();
      toast.success(`${labelFor(provider)} key saved`);
    } catch (e) {
      updateRow(provider, { saving: false });
      toast.error(`Could not save ${labelFor(provider)} key`, {
        description: humanizeError(e),
      });
    }
  };

  const onDeleteKey = async (provider: ProviderId) => {
    if (!window.confirm(`Remove your ${labelFor(provider)} API key?`)) return;
    try {
      await deleteProviderKey(provider);
      updateRow(provider, { testResult: null, testError: null });
      await refresh();
      toast.success(`${labelFor(provider)} key removed`);
    } catch (e) {
      toast.error(`Could not remove ${labelFor(provider)} key`, {
        description: humanizeError(e),
      });
    }
  };

  const onTest = async (provider: ProviderId) => {
    updateRow(provider, { testing: true, testResult: null, testError: null });
    try {
      await testProvider(provider);
      updateRow(provider, { testing: false, testResult: "ok" });
      toast.success(`${labelFor(provider)} key works`);
    } catch (e) {
      const msg = humanizeError(e);
      updateRow(provider, {
        testing: false,
        testResult: "fail",
        testError: msg,
      });
      toast.error(`${labelFor(provider)} test failed`, { description: msg });
    }
  };

  return (
    <section className="space-y-6">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">AI providers</h2>
        <p className="text-sm text-muted-foreground">
          Bring your own API key. Stored in the macOS Keychain on this machine only.
          Used to summarise meetings, extract tasks, and chat with transcripts.
        </p>
      </header>

      <AutoAgentsCard
        settings={settings}
        onChange={onChange}
        hasAiKey={providers?.some((p) => p.id === "openai" && p.configured) ?? false}
      />

      <BriefingLanguageCard settings={settings} onChange={onChange} />

      {providers === null ? (
        <p className="text-sm text-muted-foreground">Loading providers…</p>
      ) : (
        <div className="space-y-3">
          {providers.map((p) => (
            <ProviderRow
              key={p.id}
              provider={p}
              state={rowState(p.id)}
              onChangePendingKey={(value) =>
                updateRow(p.id, { pendingKey: value, testResult: null })
              }
              onSave={() => onSaveKey(p.id)}
              onDelete={() => onDeleteKey(p.id)}
              onTest={() => onTest(p.id)}
            />
          ))}
        </div>
      )}

      <p className="text-xs text-muted-foreground">
        Phase 1 ships provider configuration only. Chat UI, agent library, and the
        per-recording <em>Summarize</em> button arrive in subsequent updates. The full
        plan is tracked at{" "}
        <code className="rounded bg-muted px-1 py-0.5 text-2xs">
          projects/folio/plan/ai-chat-multi-provider.md
        </code>{" "}
        in your vault.
      </p>
    </section>
  );
}

function AutoAgentsCard({
  settings,
  onChange,
  hasAiKey,
}: {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
  hasAiKey: boolean;
}) {
  const [showDetails, setShowDetails] = React.useState(false);
  const masterOn =
    settings.auto_summarize_enabled ||
    settings.auto_extract_tasks_enabled ||
    settings.auto_extract_memories_enabled ||
    settings.auto_name_enabled;

  const setAll = (on: boolean) => {
    onChange("auto_summarize_enabled", on);
    onChange("auto_extract_tasks_enabled", on);
    onChange("auto_extract_memories_enabled", on);
    onChange("auto_name_enabled", on);
  };

  return (
    <div className="space-y-2 rounded-lg border border-border bg-card p-4">
      <div className="flex items-start justify-between gap-6">
        <div className="space-y-1">
          <Label
            htmlFor="ai-master-toggle"
            className="flex items-center gap-2 text-sm font-medium"
          >
            <Zap className="h-4 w-4 text-muted-foreground" />
            AI on every recording
            <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
              Recommended
            </span>
          </Label>
          <p className="max-w-md text-xs text-muted-foreground">
            After each recording, Folio automatically runs the Summarize, Extract Tasks,
            and Extract Memories agents in parallel. Expand below to disable individual
            agents.
          </p>
          {!hasAiKey ? (
            <p className="text-2xs font-medium text-amber-600 dark:text-amber-400">
              Add an OpenAI API key below to turn this on.
            </p>
          ) : null}
        </div>
        <Switch
          id="ai-master-toggle"
          checked={masterOn && hasAiKey}
          disabled={!hasAiKey}
          onCheckedChange={setAll}
          className="mt-1"
        />
      </div>

      <button
        type="button"
        onClick={() => setShowDetails((s) => !s)}
        className="text-2xs uppercase tracking-wider text-muted-foreground hover:text-foreground"
      >
        {showDetails ? "▾" : "▸"} Per-agent overrides
      </button>

      {showDetails ? (
        <div className="mt-2 space-y-2 border-t border-border pt-3">
          <PerAgentRow
            id="auto-summarize-toggle"
            label="Summarize"
            description="One-paragraph summary + bulleted highlights."
            checked={settings.auto_summarize_enabled && hasAiKey}
            disabled={!hasAiKey}
            onChange={(v) => onChange("auto_summarize_enabled", v)}
          />
          <PerAgentRow
            id="auto-extract-tasks-toggle"
            label="Extract tasks"
            description="Action items land on the kanban, linked back to this recording."
            checked={settings.auto_extract_tasks_enabled && hasAiKey}
            disabled={!hasAiKey}
            onChange={(v) => onChange("auto_extract_tasks_enabled", v)}
          />
          <PerAgentRow
            id="auto-extract-memories-toggle"
            label="Extract memories"
            description="Lasting facts (your projects, the people you mention) join the Memory page and get injected into future agent runs."
            checked={settings.auto_extract_memories_enabled && hasAiKey}
            disabled={!hasAiKey}
            onChange={(v) => onChange("auto_extract_memories_enabled", v)}
          />
          <PerAgentRow
            id="auto-name-toggle"
            label="Auto-name"
            description="Propose a short title, 1-3 tags, and a one-line subtitle on every recording. Shown as a hint under the row in the Library."
            checked={settings.auto_name_enabled && hasAiKey}
            disabled={!hasAiKey}
            onChange={(v) => onChange("auto_name_enabled", v)}
          />
        </div>
      ) : null}
    </div>
  );
}

const BRIEFING_LANGUAGES: { value: string; label: string }[] = [
  { value: "auto", label: "Auto (match meeting)" },
  { value: "en", label: "English" },
  { value: "tr", label: "Turkish" },
  { value: "az", label: "Azerbaijani" },
  { value: "ru", label: "Russian" },
  { value: "de", label: "German" },
  { value: "es", label: "Spanish" },
  { value: "fr", label: "French" },
  { value: "it", label: "Italian" },
  { value: "pt", label: "Portuguese" },
  { value: "nl", label: "Dutch" },
  { value: "pl", label: "Polish" },
  { value: "ar", label: "Arabic" },
  { value: "ja", label: "Japanese" },
  { value: "zh", label: "Chinese" },
];

function BriefingLanguageCard({
  settings,
  onChange,
}: {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}) {
  return (
    <div className="space-y-3 rounded-lg border border-border bg-card p-4">
      <div className="space-y-1">
        <Label
          htmlFor="briefing-language-select"
          className="flex items-center gap-2 text-sm font-medium"
        >
          <Sparkles className="h-4 w-4 text-muted-foreground" />
          Briefing language
        </Label>
        <p className="max-w-md text-xs text-muted-foreground">
          Summaries, extracted tasks, memories, and auto-names are written in this
          language regardless of the meeting&apos;s language. Quoted evidence snippets
          stay in the transcript&apos;s original language.
        </p>
      </div>
      <select
        id="briefing-language-select"
        value={settings.briefing_language}
        onChange={(e) => onChange("briefing_language", e.target.value)}
        className="h-9 w-full rounded-md border border-input bg-card px-3 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        {BRIEFING_LANGUAGES.map((l) => (
          <option key={l.value} value={l.value}>
            {l.label}
          </option>
        ))}
      </select>
    </div>
  );
}

function PerAgentRow({
  id,
  label,
  description,
  checked,
  disabled,
  onChange,
}: {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="space-y-0.5">
        <Label htmlFor={id} className="text-xs font-medium">
          {label}
        </Label>
        <p className="max-w-md text-2xs text-muted-foreground">{description}</p>
      </div>
      <Switch
        id={id}
        checked={checked}
        disabled={disabled}
        onCheckedChange={onChange}
        className="mt-0.5"
      />
    </div>
  );
}

interface ProviderRowProps {
  provider: ProviderStatus;
  state: ProviderRowState;
  onChangePendingKey: (value: string) => void;
  onSave: () => void;
  onDelete: () => void;
  onTest: () => void;
}

function ProviderRow({
  provider,
  state,
  onChangePendingKey,
  onSave,
  onDelete,
  onTest,
}: ProviderRowProps) {
  const [revealing, setRevealing] = React.useState(false);
  const inputId = `provider-key-${provider.id}`;

  return (
    <div
      className={cn(
        "rounded-lg border border-border bg-card p-4",
        provider.recommended && "ring-1 ring-primary/40"
      )}
    >
      <div className="mb-3 flex items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Sparkles className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-semibold">{provider.display_name}</span>
          {provider.recommended ? (
            <Badge variant="secondary" className="text-2xs">
              Recommended
            </Badge>
          ) : null}
        </div>
        <StatusPill configured={provider.configured} testResult={state.testResult} />
      </div>

      <div className="space-y-2">
        <Label
          htmlFor={inputId}
          className="flex items-center gap-1.5 text-xs text-muted-foreground"
        >
          <KeyRound className="h-3 w-3" />
          API key
          {provider.configured ? (
            <span className="ml-1 font-mono text-foreground">
              {provider.redacted_suffix ?? ""}
            </span>
          ) : null}
        </Label>
        <div className="flex items-stretch gap-2">
          <Input
            id={inputId}
            type={revealing ? "text" : "password"}
            value={state.pendingKey}
            onChange={(e) => onChangePendingKey(e.target.value)}
            placeholder={provider.configured ? "Paste a new key to replace" : "sk-…"}
            className="font-mono text-xs"
            autoComplete="off"
            spellCheck={false}
          />
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => setRevealing((v) => !v)}
            disabled={!state.pendingKey}
          >
            {revealing ? "Hide" : "Show"}
          </Button>
        </div>

        <div className="flex flex-wrap items-center gap-2 pt-1">
          <Button
            type="button"
            size="sm"
            onClick={onSave}
            disabled={!state.pendingKey || state.saving}
          >
            {state.saving ? (
              <>
                <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                Saving…
              </>
            ) : provider.configured ? (
              "Replace"
            ) : (
              "Save key"
            )}
          </Button>
          {provider.configured ? (
            <>
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={onTest}
                disabled={state.testing}
              >
                {state.testing ? (
                  <>
                    <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                    Testing…
                  </>
                ) : (
                  "Test"
                )}
              </Button>
              <Button
                type="button"
                size="sm"
                variant="ghost"
                onClick={onDelete}
                className="text-destructive hover:text-destructive"
              >
                Remove
              </Button>
            </>
          ) : null}
        </div>

        {state.testError ? (
          <p className="pt-1 text-2xs text-destructive">{state.testError}</p>
        ) : null}
      </div>
    </div>
  );
}

function StatusPill({
  configured,
  testResult,
}: {
  configured: boolean;
  testResult: "ok" | "fail" | null;
}) {
  if (testResult === "ok") {
    return (
      <Badge variant="secondary" className="gap-1">
        <CheckCircle2 className="h-3 w-3 text-emerald-500" />
        Verified
      </Badge>
    );
  }
  if (testResult === "fail") {
    return (
      <Badge variant="destructive" className="gap-1">
        <XCircle className="h-3 w-3" />
        Test failed
      </Badge>
    );
  }
  if (configured) {
    return (
      <Badge variant="secondary" className="gap-1">
        <CheckCircle2 className="h-3 w-3 text-emerald-500" />
        Configured
      </Badge>
    );
  }
  return (
    <Badge variant="outline" className="text-2xs">
      Not configured
    </Badge>
  );
}

function labelFor(id: ProviderId): string {
  switch (id) {
    case "openai":
      return "OpenAI";
  }
}
