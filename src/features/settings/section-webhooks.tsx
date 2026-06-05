import * as React from "react";
import { Loader2, Plug, Send, Trash2 } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import { cn } from "@/shared/lib/utils";
import { humanizeError } from "@/shared/lib/errors";
import {
  deleteWebhook,
  listWebhooks,
  saveWebhook,
  testWebhook,
} from "@/shared/lib/ipc";
import type { WebhookEvent } from "@/shared/types/WebhookEvent";
import type { WebhookSubscription } from "@/shared/types/WebhookSubscription";

const EVENTS: { id: WebhookEvent; label: string }[] = [
  { id: "recording_finished", label: "recording.finished" },
  { id: "transcript_ready", label: "transcript.ready" },
  { id: "task_created", label: "task.created" },
  { id: "memory_created", label: "memory.created" },
];

export function SectionWebhooks() {
  const [subs, setSubs] = React.useState<WebhookSubscription[]>([]);
  const [loading, setLoading] = React.useState(true);
  const [draft, setDraft] = React.useState<WebhookSubscription | null>(null);
  const [testing, setTesting] = React.useState<string | null>(null);

  const refresh = React.useCallback(async () => {
    setLoading(true);
    try {
      setSubs(await listWebhooks());
    } catch (e) {
      console.error("list_webhooks:", e);
      toast.error("Could not load webhooks", { description: humanizeError(e) });
    } finally {
      setLoading(false);
    }
  }, []);

  React.useEffect(() => {
    refresh();
  }, [refresh]);

  const startNew = () => {
    setDraft({
      id: "",
      label: "",
      url: "http://localhost:9000/folio",
      secret: cryptoRandomSecret(),
      events: [],
      enabled: true,
    });
  };

  const handleSave = async () => {
    if (!draft) return;
    try {
      const saved = await saveWebhook(draft);
      setSubs((cur) => {
        const without = cur.filter((s) => s.id !== saved.id);
        return [...without, saved];
      });
      setDraft(null);
      toast.success("Webhook saved");
    } catch (e) {
      console.error("save_webhook:", e);
      toast.error("Could not save webhook", { description: humanizeError(e) });
    }
  };

  const handleDelete = async (id: string) => {
    if (!window.confirm("Remove this webhook subscription?")) return;
    try {
      await deleteWebhook(id);
      setSubs((cur) => cur.filter((s) => s.id !== id));
      toast.success("Webhook removed");
    } catch (e) {
      console.error("delete_webhook:", e);
      toast.error("Could not delete webhook", { description: humanizeError(e) });
    }
  };

  const handleTest = async (id: string) => {
    setTesting(id);
    try {
      const status = await testWebhook(id);
      toast.success("Test event delivered", { description: status });
    } catch (e) {
      console.error("test_webhook:", e);
      toast.error("Webhook test failed", { description: humanizeError(e) });
    } finally {
      setTesting(null);
    }
  };

  const toggleEnabled = async (sub: WebhookSubscription) => {
    const next = { ...sub, enabled: !sub.enabled };
    try {
      const saved = await saveWebhook(next);
      setSubs((cur) => cur.map((s) => (s.id === saved.id ? saved : s)));
    } catch (e) {
      console.error("toggle webhook enabled:", e);
      toast.error("Could not update webhook", { description: humanizeError(e) });
    }
  };

  return (
    <div className="flex flex-col gap-7">
      <div>
        <h2 className="font-serif text-2xl font-medium">Webhooks</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Forward lifecycle events to local endpoints with an HMAC-SHA256 signature.
          Drives note-taking syncs, automation flows, dashboards, and anything else that
          wants to react when Folio captures a recording.
        </p>
      </div>

      {loading ? (
        <p className="text-sm text-muted-foreground">Loading…</p>
      ) : subs.length === 0 ? (
        <p className="rounded-lg border border-dashed border-border bg-card px-4 py-6 text-center text-xs text-muted-foreground">
          No subscriptions yet. Add one to start receiving signed events.
        </p>
      ) : (
        <ul className="flex flex-col gap-2">
          {subs.map((sub) => (
            <li
              key={sub.id}
              className={cn(
                "flex flex-col gap-2 rounded-lg border border-border bg-card p-3",
                !sub.enabled && "opacity-60"
              )}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <Plug className="h-3.5 w-3.5 text-muted-foreground" />
                    <span className="truncate text-sm font-medium">{sub.label}</span>
                  </div>
                  <p className="mt-0.5 truncate font-mono text-2xs text-muted-foreground">
                    {sub.url}
                  </p>
                  {sub.events.length > 0 && (
                    <p className="mt-0.5 truncate text-2xs text-muted-foreground">
                      {sub.events.join("  ·  ")}
                    </p>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  <Switch
                    checked={sub.enabled}
                    onCheckedChange={() => toggleEnabled(sub)}
                    aria-label="Enabled"
                  />
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 text-muted-foreground hover:text-foreground"
                    onClick={() => handleTest(sub.id)}
                    disabled={testing === sub.id}
                    title="Send a test event"
                    aria-label="Test webhook"
                  >
                    {testing === sub.id ? (
                      <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    ) : (
                      <Send className="h-3.5 w-3.5" />
                    )}
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-7 w-7 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
                    onClick={() => handleDelete(sub.id)}
                    title="Remove"
                    aria-label="Remove webhook"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}

      {draft ? (
        <div className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4">
          <Label className="text-xs">
            Label
            <Input
              value={draft.label}
              onChange={(e) => setDraft({ ...draft, label: e.target.value })}
              placeholder="Notion sync"
              className="mt-1"
            />
          </Label>
          <Label className="text-xs">
            URL
            <Input
              value={draft.url}
              onChange={(e) => setDraft({ ...draft, url: e.target.value })}
              placeholder="http://localhost:9000/folio"
              className="mt-1 font-mono text-xs"
            />
          </Label>
          <Label className="text-xs">
            Secret (HMAC-SHA256)
            <Input
              value={draft.secret}
              onChange={(e) => setDraft({ ...draft, secret: e.target.value })}
              className="mt-1 font-mono text-xs"
            />
          </Label>
          <fieldset className="flex flex-col gap-1.5">
            <legend className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
              Events (none = all)
            </legend>
            {EVENTS.map((e) => {
              const on = draft.events.includes(e.id);
              return (
                <label key={e.id} className="flex items-center gap-2 text-xs">
                  <input
                    type="checkbox"
                    checked={on}
                    onChange={(ev) => {
                      const events = ev.target.checked
                        ? [...draft.events, e.id]
                        : draft.events.filter((x) => x !== e.id);
                      setDraft({ ...draft, events });
                    }}
                  />
                  <span className="font-mono">{e.label}</span>
                </label>
              );
            })}
          </fieldset>
          <div className="flex justify-end gap-2">
            <Button variant="ghost" size="sm" onClick={() => setDraft(null)}>
              Cancel
            </Button>
            <Button size="sm" onClick={handleSave}>
              Save subscription
            </Button>
          </div>
        </div>
      ) : (
        <div className="flex justify-start">
          <Button variant="outline" size="sm" onClick={startNew} className="gap-2">
            <Plug className="h-3.5 w-3.5" />
            Add subscription
          </Button>
        </div>
      )}
    </div>
  );
}

function cryptoRandomSecret(): string {
  const bytes = new Uint8Array(16);
  if (typeof crypto !== "undefined" && crypto.getRandomValues) {
    crypto.getRandomValues(bytes);
  } else {
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = Math.floor(Math.random() * 256);
    }
  }
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}
