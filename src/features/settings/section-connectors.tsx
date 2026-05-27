/**
 * GET-136 — Settings → Connectors.
 *
 * Single-column layout led by the localhost MCP card (the signature
 * Attune inversion of Granola's cloud-routed MCP). Below it: tiles
 * for Slack / Notion / Linear / Gmail OAuth integrations plus the
 * already-shipped Apple Reminders / Apple Calendar surfaces.
 *
 * v1 ships the UI scaffold + copy-the-localhost-URL affordance. The
 * actual MCP server binding and per-connector OAuth flows are
 * separate epics; clicking Connect on any cloud tile shows a toast
 * acknowledging the deferral. The MCP card surfaces a "coming soon"
 * status until the server lands so the copy-URL button doesn't
 * mislead users into pointing Claude/Cursor at a dead port.
 */

import * as React from "react";
import {
  ArrowUpRight,
  Calendar as CalendarIcon,
  Check,
  Copy,
  ExternalLink,
  FileText,
  Inbox,
  KeyRound,
  Layers,
  Mail,
  MessageSquare,
  Plus,
  Workflow,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import { cn } from "@/shared/lib/utils";

const MCP_DEFAULT_URL = "http://127.0.0.1:7438/mcp";

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
    id: "apple-calendar",
    name: "Apple Calendar",
    description:
      "Pre-fills meeting titles + attendees, detects when meetings start, and auto-names recordings.",
    icon: CalendarIcon,
    status: "shipped",
    shippedNote: "Granted in Settings → Calendar",
  },
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
    description: "Pull recent threads with attendees to brief Attune before meetings.",
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
          Where Attune sends meeting data, and which AI tools can ask Attune
          about your past meetings.
        </p>
      </header>

      <McpFeatureCard />

      <Group title="Integrations">
        <div className="space-y-2">
          {CONNECTORS.map((c) => (
            <ConnectorRow key={c.id} card={c} />
          ))}
        </div>
      </Group>

      <ApiKeysStub />
    </section>
  );
}

function McpFeatureCard() {
  const [copied, setCopied] = React.useState(false);

  const copyUrl = async () => {
    try {
      await navigator.clipboard.writeText(MCP_DEFAULT_URL);
      setCopied(true);
      toast.success("MCP URL copied", {
        description: "Paste into Claude Desktop, Cursor, or any MCP-aware tool.",
      });
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error("clipboard:", e);
      toast.error("Could not copy URL", { description: String(e) });
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
            <span className="rounded-full bg-amber-100 px-2 py-0.5 text-2xs font-medium uppercase tracking-wider text-amber-800 dark:bg-amber-950/40 dark:text-amber-200">
              Coming soon
            </span>
          </div>
          <p className="mt-1 max-w-prose text-sm text-muted-foreground">
            Your meetings, queryable from every AI tool on your Mac. The server
            runs on localhost — no cloud, no proxy. Claude Desktop, Cursor,
            Raycast, and any other MCP-aware client connects directly.
          </p>
        </div>
      </div>

      <div className="mt-5 flex items-center gap-2 rounded-lg border border-border bg-background px-3 py-2">
        <code className="flex-1 truncate font-mono text-xs text-foreground">
          {MCP_DEFAULT_URL}
        </code>
        <Button type="button" size="sm" variant="outline" onClick={copyUrl} className="gap-1.5">
          {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
          {copied ? "Copied" : "Copy"}
        </Button>
      </div>

      <ol className="mt-5 space-y-2 text-sm">
        <Step n={1}>
          Add the URL above to your AI tool (Claude Desktop &rarr; Settings &rarr;
          Developer &rarr; Edit Config).
        </Step>
        <Step n={2}>
          Approve the consent prompt the first time the tool queries Attune.
        </Step>
        <Step n={3}>
          Chat anywhere with meeting context — search transcripts, find
          decisions, fetch action items.
        </Step>
      </ol>

      <div className="mt-4 flex items-center justify-between">
        <button
          type="button"
          className="inline-flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
          onClick={() =>
            toast.info("Audit log", {
              description: "Per-query consent log lands with the MCP server.",
            })
          }
        >
          View what was queried
          <ArrowUpRight className="h-3 w-3" />
        </button>
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

function Step({ n, children }: { n: number; children: React.ReactNode }) {
  return (
    <li className="flex items-start gap-3">
      <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 text-2xs font-medium text-primary">
        {n}
      </span>
      <span className="text-muted-foreground">{children}</span>
    </li>
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
        "OAuth flow ships with the attune-api backend (GET-123). The tile is in place so it lights up automatically when ready.",
    });
  };

  return (
    <div
      className={cn(
        "flex items-start gap-4 rounded-lg border bg-card p-4 transition-colors",
        shipped ? "border-border" : "border-dashed border-border",
      )}
    >
      <div
        className={cn(
          "flex h-10 w-10 shrink-0 items-center justify-center rounded-md",
          shipped ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground",
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

function ApiKeysStub() {
  return (
    <Group title="API keys">
      <div className="flex items-start gap-4 rounded-lg border border-dashed border-border bg-card p-4">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-muted text-muted-foreground">
          <KeyRound className="h-4.5 w-4.5" />
        </div>
        <div className="min-w-0 flex-1 space-y-0.5">
          <p className="text-sm font-medium">Personal access tokens</p>
          <p className="max-w-prose text-xs text-muted-foreground">
            For scripts and CI that need to query Attune programmatically.
            Each token is scoped to one workspace and revocable from this
            page.
          </p>
          <p className="text-2xs text-muted-foreground/80">
            Lands alongside the attune-api auth surface.
          </p>
        </div>
        <Button
          type="button"
          size="sm"
          variant="outline"
          disabled
          className="mt-0.5 shrink-0 gap-1.5"
        >
          <Plus className="h-3.5 w-3.5" />
          New token
        </Button>
      </div>
    </Group>
  );
}

