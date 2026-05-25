import * as React from "react";
import { CheckCircle2, KeyRound, Loader2, Sparkles, XCircle, Zap } from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
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

/**
 * Phase 1 of the AI chat feature. Lets the user paste an API key for
 * each supported provider, store it in the macOS Keychain, and run a
 * lightweight Test call to confirm the key works. Chat UI lands in
 * later phases.
 *
 * The vault plan is at:
 *   ~/Documents/GitHub/obsidian.md/projects/attune/plan/ai-chat-multi-provider.md
 */
export function SectionAi({ settings, onChange }: SectionAiProps) {
  const [providers, setProviders] = React.useState<ProviderStatus[] | null>(null);
  const [rows, setRows] = React.useState<Record<string, ProviderRowState>>({});

  const refresh = React.useCallback(async () => {
    try {
      const list = await listProviders();
      setProviders(list);
    } catch (e) {
      console.error("listProviders:", e);
      toast.error("Could not load AI providers", { description: String(e) });
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
        description: String(e),
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
        description: String(e),
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
      const msg = String(e);
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
        <h2 className="text-lg font-semibold">AI providers</h2>
        <p className="text-sm text-muted-foreground">
          Bring your own API key. Stored in the macOS Keychain on this machine only.
          Used to summarise meetings, extract tasks, and chat with transcripts.
        </p>
      </header>

      {/* Auto-summarize toggle. Reads from + writes to Settings; the
          recording-store consults this after each transcription and
          fires the Summarize agent in the background when on. */}
      <div className="flex items-start justify-between gap-6 rounded-lg border border-border bg-card p-4">
        <div className="space-y-1">
          <Label
            htmlFor="auto-summarize-toggle"
            className="flex items-center gap-2 text-sm font-medium"
          >
            <Zap className="h-4 w-4 text-muted-foreground" />
            Auto-summarize after recording
            <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
              Recommended
            </span>
          </Label>
          <p className="max-w-md text-xs text-muted-foreground">
            When a recording finishes transcribing, run the Summarize agent
            automatically. The summary appears on the recording&apos;s page and in the
            AI tab the next time you open them. Skipped if no AI key is set.
          </p>
        </div>
        <Switch
          id="auto-summarize-toggle"
          checked={settings.auto_summarize_enabled}
          onCheckedChange={(checked) => onChange("auto_summarize_enabled", checked)}
          className="mt-1"
        />
      </div>

      {/* Auto-extract-tasks toggle. The Extract Tasks agent uses tool
          calling to write directly into the kanban, so this fires
          alongside auto-summarize after each transcription. */}
      <div className="flex items-start justify-between gap-6 rounded-lg border border-border bg-card p-4">
        <div className="space-y-1">
          <Label
            htmlFor="auto-extract-tasks-toggle"
            className="flex items-center gap-2 text-sm font-medium"
          >
            <Zap className="h-4 w-4 text-muted-foreground" />
            Auto-extract tasks after recording
            <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
              Recommended
            </span>
          </Label>
          <p className="max-w-md text-xs text-muted-foreground">
            Run the Extract Tasks agent automatically once a recording is transcribed.
            Action items the agent finds land directly on your kanban with a link back
            to the source meeting. Skipped if no AI key is set.
          </p>
        </div>
        <Switch
          id="auto-extract-tasks-toggle"
          checked={settings.auto_extract_tasks_enabled}
          onCheckedChange={(checked) => onChange("auto_extract_tasks_enabled", checked)}
          className="mt-1"
        />
      </div>

      {/* Auto-extract-memories toggle. The Extract Memories agent uses
          the `remember` tool to capture lasting facts about the user
          (identity, projects, preferences, people) so future agent
          runs get them injected as background context. */}
      <div className="flex items-start justify-between gap-6 rounded-lg border border-border bg-card p-4">
        <div className="space-y-1">
          <Label
            htmlFor="auto-extract-memories-toggle"
            className="flex items-center gap-2 text-sm font-medium"
          >
            <Zap className="h-4 w-4 text-muted-foreground" />
            Auto-extract memories after recording
            <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
              Recommended
            </span>
          </Label>
          <p className="max-w-md text-xs text-muted-foreground">
            Run the Extract Memories agent so lasting facts (your role, company,
            projects, the people you work with, your preferences) land on the Memory
            page automatically. The next time you run any agent, those facts get
            injected as background context so it doesn&apos;t re-ask things you&apos;ve
            already said.
          </p>
        </div>
        <Switch
          id="auto-extract-memories-toggle"
          checked={settings.auto_extract_memories_enabled}
          onCheckedChange={(checked) =>
            onChange("auto_extract_memories_enabled", checked)
          }
          className="mt-1"
        />
      </div>

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
          projects/attune/plan/ai-chat-multi-provider.md
        </code>{" "}
        in your vault.
      </p>
    </section>
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
    case "anthropic":
      return "Anthropic";
    case "deepseek":
      return "DeepSeek";
  }
}
