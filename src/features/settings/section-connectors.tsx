import * as React from "react";
import {
  Check,
  CheckCircle2,
  Copy,
  ExternalLink,
  FileText,
  Inbox,
  Layers,
  Loader2,
  Mail,
  MessageSquare,
  Plus,
  Terminal,
  Workflow,
  XCircle,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import { cn } from "@/shared/lib/utils";
import { humanizeError } from "@/shared/lib/errors";
import {
  generateMcpConfig,
  grantMcpClient,
  listMcpAccessLog,
  listMcpGrants,
  revokeMcpClient,
  writeMcpConfig,
  type McpAccessEntry,
  type McpClient,
  type McpClientGrant,
  type McpConnectInfo,
} from "@/shared/lib/ipc";

type ConnectorStatus = "shipped" | "coming_soon";

interface ConnectorCard {
  id: string;
  name: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  status: ConnectorStatus;
  shippedNote?: string;
}

const CONNECTORS: ConnectorCard[] = [
  {
    id: "apple-reminders",
    name: "Apple Reminders",
    description:
      "Push extracted action items to your Reminders list so they show up in Today.",
    icon: Inbox,
    status: "shipped",
    shippedNote: "Configure in Settings → AI",
  },
  {
    id: "slack",
    name: "Slack",
    description: "Share meeting summaries to channels or DMs with one click.",
    icon: MessageSquare,
    status: "coming_soon",
  },
  {
    id: "notion",
    name: "Notion",
    description: "Export meeting notes to a Notion database as fully-formatted pages.",
    icon: FileText,
    status: "coming_soon",
  },
  {
    id: "linear",
    name: "Linear",
    description: "Push action items straight into a Linear team as issues.",
    icon: Layers,
    status: "coming_soon",
  },
  {
    id: "gmail",
    name: "Gmail",
    description: "Pull recent threads with attendees to brief Folio before meetings.",
    icon: Mail,
    status: "coming_soon",
  },
];

export function SectionConnectors() {
  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Connectors</h2>
        <p className="text-sm text-muted-foreground">
          Where Folio sends meeting data, and which AI tools can ask Folio about your
          past meetings.
        </p>
      </header>

      <McpFeatureCard />

      <McpConsentPanel />

      <Group title="Integrations">
        <div className="space-y-2">
          {CONNECTORS.map((c) => (
            <ConnectorRow key={c.id} card={c} />
          ))}
        </div>
      </Group>
    </section>
  );
}

function McpConsentPanel() {
  const [grants, setGrants] = React.useState<McpClientGrant[]>([]);
  const [log, setLog] = React.useState<McpAccessEntry[]>([]);
  const [logOpen, setLogOpen] = React.useState(false);
  const [busy, setBusy] = React.useState<string | null>(null);

  const reload = React.useCallback(() => {
    listMcpGrants()
      .then(setGrants)
      .catch(() => {});
    if (logOpen) {
      listMcpAccessLog()
        .then(setLog)
        .catch(() => {});
    }
  }, [logOpen]);

  React.useEffect(() => {
    reload();
  }, [reload]);

  React.useEffect(() => {
    if (logOpen) {
      listMcpAccessLog()
        .then(setLog)
        .catch(() => {});
    }
  }, [logOpen]);

  const toggle = async (grant: McpClientGrant) => {
    setBusy(grant.client_id);
    try {
      if (grant.allow_reads) {
        await revokeMcpClient(grant.client_id);
      } else {
        await grantMcpClient(grant.client_id, grant.client_name ?? undefined);
      }
      reload();
    } catch (e) {
      console.error("mcp_grant toggle:", e);
    } finally {
      setBusy(null);
    }
  };

  if (grants.length === 0 && log.length === 0) return null;

  return (
    <div className="space-y-3">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        MCP access control
      </Label>

      {grants.length > 0 ? (
        <div className="space-y-2">
          {grants.map((g) => (
            <div
              key={g.client_id}
              className="flex items-center justify-between rounded-lg border border-border bg-card px-3 py-2"
            >
              <div>
                <p className="text-sm font-medium">{g.client_name ?? g.client_id}</p>
                <p className="text-2xs text-muted-foreground">
                  {g.allow_reads ? "Read access granted" : "Access revoked"}
                  {g.granted_at
                    ? ` · ${new Date(g.granted_at).toLocaleDateString([], { month: "short", day: "numeric" })}`
                    : ""}
                </p>
              </div>
              <Button
                size="sm"
                variant={g.allow_reads ? "destructive" : "outline"}
                className="gap-1.5"
                disabled={busy === g.client_id}
                onClick={() => void toggle(g)}
              >
                {g.allow_reads ? "Revoke" : "Grant"}
              </Button>
            </div>
          ))}
        </div>
      ) : null}

      <button
        type="button"
        onClick={() => setLogOpen((v) => !v)}
        className="text-xs text-muted-foreground hover:text-foreground"
      >
        {logOpen ? "▾" : "▸"} Access log ({log.length} entries)
      </button>

      {logOpen && log.length > 0 ? (
        <div className="max-h-48 overflow-y-auto rounded-md border border-border bg-muted/30 p-2 font-mono text-2xs">
          {log.map((e, i) => (
            <div key={i} className="py-0.5 text-muted-foreground">
              <span className="text-foreground/70">
                {new Date(e.ts).toLocaleTimeString([], {
                  hour: "2-digit",
                  minute: "2-digit",
                })}
              </span>{" "}
              <span className="text-primary/80">{e.client}</span> →{" "}
              <span>{e.tool}</span>
              {e.notes.length > 0 ? (
                <span className="ml-1 text-muted-foreground/60">
                  [{e.notes.length} note{e.notes.length !== 1 ? "s" : ""}]
                </span>
              ) : null}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function McpFeatureCard() {
  const [info, setInfo] = React.useState<McpConnectInfo | null>(null);
  const [loading, setLoading] = React.useState(true);
  const [writtenIds, setWrittenIds] = React.useState<Set<string>>(new Set());

  React.useEffect(() => {
    generateMcpConfig()
      .then(setInfo)
      .catch((e) => console.error("generate_mcp_config:", e))
      .finally(() => setLoading(false));
  }, []);

  const copySnippet = async (client: McpClient) => {
    try {
      const text = client.cli_command ?? client.json_snippet;
      await navigator.clipboard.writeText(text);
      toast.success(`${client.name} config copied`);
    } catch (e) {
      toast.error("Could not copy", { description: humanizeError(e) });
    }
  };

  const writeConfig = async (client: McpClient) => {
    if (!client.config_path || !info?.binary_path) return;
    try {
      await writeMcpConfig(client.config_path, info.binary_path, client.id);
      setWrittenIds((prev) => new Set(prev).add(client.id));
      toast.success(`Folio added to ${client.name}`, {
        description: `Restart ${client.name} to load the new server.`,
      });
    } catch (e) {
      toast.error(`Could not write ${client.name} config`, {
        description: humanizeError(e),
      });
    }
  };

  return (
    <div className="relative overflow-hidden rounded-xl border border-primary/30 bg-gradient-to-br from-primary/5 via-card to-card p-6">
      <div className="flex items-start gap-3">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <Workflow className="h-5 w-5" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="font-serif text-lg font-medium">Local MCP server</h3>
            <span className="rounded-full bg-primary/10 px-2 py-0.5 text-2xs font-medium uppercase tracking-wider text-primary">
              Featured
            </span>
          </div>
          <p className="mt-1 max-w-prose text-sm text-muted-foreground">
            Connect any MCP-aware AI tool to Folio — no cloud, no proxy. Each tool gets
            read-only access to your transcripts, memories, and tasks via a local stdio
            server.
          </p>
        </div>
      </div>

      {loading ? (
        <div className="mt-5 flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          Detecting installed clients…
        </div>
      ) : (
        <div className="mt-5 space-y-3">
          {(info?.clients ?? []).map((client) => {
            const detected = client.status === "detected";
            const written = writtenIds.has(client.id);
            const canWrite =
              detected && client.config_path !== null && info?.binary_path !== null;
            const isCliClient = client.cli_command !== null;

            return (
              <div
                key={client.id}
                className={cn(
                  "rounded-lg border bg-card p-3",
                  detected ? "border-border" : "border-border/40 opacity-60"
                )}
              >
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    {detected ? (
                      <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-emerald-500" />
                    ) : (
                      <XCircle className="h-3.5 w-3.5 shrink-0 text-muted-foreground/40" />
                    )}
                    <span className="text-sm font-medium">{client.name}</span>
                    <span className="text-2xs text-muted-foreground">
                      {detected ? "detected" : "not installed"}
                    </span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    {isCliClient ? (
                      <Button
                        size="sm"
                        variant="outline"
                        className="gap-1.5"
                        onClick={() => void copySnippet(client)}
                        disabled={!detected}
                      >
                        <Terminal className="h-3 w-3" />
                        Copy command
                      </Button>
                    ) : (
                      <>
                        {canWrite ? (
                          <Button
                            size="sm"
                            variant={written ? "ghost" : "default"}
                            className="gap-1.5"
                            onClick={() => void writeConfig(client)}
                            disabled={written}
                          >
                            {written ? (
                              <Check className="h-3 w-3" />
                            ) : (
                              <Plus className="h-3 w-3" />
                            )}
                            {written ? "Added" : "Add Folio"}
                          </Button>
                        ) : null}
                        <Button
                          size="sm"
                          variant="outline"
                          className="gap-1.5"
                          onClick={() => void copySnippet(client)}
                        >
                          <Copy className="h-3 w-3" />
                          Copy snippet
                        </Button>
                      </>
                    )}
                  </div>
                </div>

                {detected && (
                  <pre className="mt-2 overflow-hidden whitespace-pre-wrap break-all rounded bg-muted/50 px-2 py-1.5 font-mono text-2xs leading-relaxed text-foreground/80">
                    {isCliClient
                      ? client.cli_command
                      : client.json_snippet.split("\n").slice(1).join("\n").trim()}
                  </pre>
                )}
              </div>
            );
          })}
        </div>
      )}

      <div className="mt-4 flex items-center justify-end gap-3">
        <a
          href="https://modelcontextprotocol.io"
          target="_blank"
          rel="noreferrer noopener"
          className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
        >
          What is MCP?
          <ExternalLink className="h-3 w-3" />
        </a>
      </div>
    </div>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-3">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {title}
      </Label>
      {children}
    </div>
  );
}

function ConnectorRow({ card }: { card: ConnectorCard }) {
  const Icon = card.icon;
  const shipped = card.status === "shipped";

  const handleConnect = () => {
    toast.info(`${card.name} connector`, {
      description:
        "OAuth connectors aren't available in the local build. Use webhooks or an MCP server to integrate for now.",
    });
  };

  return (
    <div
      className={cn(
        "flex items-start gap-4 rounded-lg border bg-card p-4 transition-colors",
        shipped ? "border-border" : "border-dashed border-border"
      )}
    >
      <div
        className={cn(
          "flex h-10 w-10 shrink-0 items-center justify-center rounded-md",
          shipped ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground"
        )}
      >
        <Icon className="h-4.5 w-4.5" />
      </div>
      <div className="min-w-0 flex-1 space-y-0.5">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium">{card.name}</p>
          {shipped ? (
            <span className="inline-flex items-center gap-1 rounded-full bg-green-100 px-2 py-0.5 text-2xs font-medium text-green-800 dark:bg-green-900/40 dark:text-green-200">
              <span className="h-1.5 w-1.5 rounded-full bg-green-500" />
              Connected
            </span>
          ) : (
            <span className="rounded-full bg-muted px-2 py-0.5 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
              Coming soon
            </span>
          )}
        </div>
        <p className="max-w-prose text-xs text-muted-foreground">{card.description}</p>
        {shipped && card.shippedNote ? (
          <p className="text-2xs text-muted-foreground/80">{card.shippedNote}</p>
        ) : null}
      </div>
      {shipped ? null : (
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={handleConnect}
          className="mt-0.5 shrink-0"
        >
          Connect
        </Button>
      )}
    </div>
  );
}
