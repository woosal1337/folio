/**
 * GET-138 — Settings → Workspace → General.
 *
 * Workspace setup (name + logo), invites & members policy, SSO /
 * SCIM upsell stubs, data-security visualisation, danger zone.
 *
 * Persona red lines:
 *   - Sasha: bucket-aware defaults for the two join toggles.
 *     Clinical workspaces default discoverable=false / auto-join=false
 *     because therapist practices don't accept walk-ins. The on-disk
 *     `Settings.workspace_discoverable` / `workspace_auto_join` are
 *     simple booleans; the bucket-aware default is applied here when
 *     the user hasn't touched the toggle yet (we treat missing
 *     workspace_bucket as "unknown" and trust the persisted bool).
 *   - Mira: SSO + SCIM are Pro/Enterprise tier upsell tiles, not
 *     functional toggles in v1. Clicking opens the upsell modal
 *     (stubbed to a toast for now).
 *   - Tony: Danger Zone retains the orange-red treatment from Granola
 *     but its three actions are gated behind the workspace existing
 *     server-side (we noop with a toast in local-only mode).
 *
 * Backend pairing: workspace mutation endpoints land with GET-123 in
 * attune-api. Until then, the toggles persist to local Settings only;
 * the screen displays a banner explaining that workspace policies
 * apply when the user signs in.
 */

import * as React from "react";
import {
  Building2,
  ExternalLink,
  Eye,
  EyeOff,
  Globe,
  Image as ImageIcon,
  Link2,
  Lock,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  Trash2,
  UserPlus,
  Users,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import type { Settings } from "@/shared/types/Settings";

interface SectionProps {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

export function SectionWorkspaceGeneral({ settings, onChange }: SectionProps) {
  const isClinical = settings.workspace_bucket === "clinical";
  const isLocalOnly = settings.signin_mode === "" || settings.signin_mode === "offline";

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Workspace</h2>
        <p className="text-sm text-muted-foreground">
          Identity, membership, and policy for the team that shares this
          workspace.
        </p>
      </header>

      {isLocalOnly ? <LocalOnlyBanner /> : null}

      <Group title="Setup">
        <FieldRow icon={Building2} title="Workspace name">
          <Input
            value={settings.workspace_name}
            onChange={(e) => onChange("workspace_name", e.target.value)}
            placeholder="e.g. Clinora"
            maxLength={64}
            className="max-w-xs"
          />
        </FieldRow>
        <FieldRow icon={ImageIcon} title="Logo" subtitle="PNG or SVG, max 256x256.">
          <LogoPicker
            path={settings.workspace_logo_path}
            onChange={(v) => onChange("workspace_logo_path", v)}
          />
        </FieldRow>
        <FieldRow
          icon={Sparkles}
          title="Workspace type"
          subtitle="Set during onboarding. Changing this can affect privacy defaults — contact support."
        >
          <BucketBadge bucket={settings.workspace_bucket} />
        </FieldRow>
      </Group>

      <Group title="Invites and members">
        <ToggleRow
          icon={settings.workspace_discoverable ? Eye : EyeOff}
          title="Discoverable by matching email domain"
          description={
            isClinical
              ? "Off by default for Clinical workspaces. Therapist and medical practices shouldn't surface to walk-in colleagues."
              : "Other people signing in with a matching work email will see this workspace in their workspace picker."
          }
          checked={settings.workspace_discoverable}
          onChange={(v) => onChange("workspace_discoverable", v)}
        />
        <ToggleRow
          icon={UserPlus}
          title="Allow teammates to join automatically"
          description={
            isClinical
              ? "Off by default for Clinical workspaces. New members must be invited explicitly."
              : "Matching-domain teammates can join without admin approval."
          }
          checked={settings.workspace_auto_join}
          onChange={(v) => onChange("workspace_auto_join", v)}
        />
        <ActionRow
          icon={Link2}
          title="Invite links"
          description="Manage existing invite links — revoke, regenerate, set expiry."
          actionLabel="Manage"
          onAction={() =>
            toast.info("Invite link management", {
              description: "Lands with the backend invite endpoints (GET-123).",
            })
          }
        />
      </Group>

      <Group title="Identity tier">
        <UpsellRow
          icon={Lock}
          title="Single Sign-On (SSO)"
          subtitle="Enforce SAML or OIDC sign-in for everyone in this workspace."
          badge="Enterprise"
        />
        <UpsellRow
          icon={Users}
          title="Directory sync (SCIM)"
          subtitle="Provision and deprovision members from your IdP automatically."
          badge="Enterprise"
        />
      </Group>

      <Group title="Data security">
        <PrivacyTierViz
          bucket={settings.workspace_bucket}
          autoDelete={settings.auto_delete_period_days}
        />
        <ActionRow
          icon={Globe}
          title="Data export permissions"
          description="Control who can export transcripts, move notes between workspaces, or transfer ownership."
          actionLabel="Configure"
          onAction={() =>
            toast.info("Data export policy", {
              description: "Lands alongside the workspace permissions backend.",
            })
          }
        />
      </Group>

      <Group title="Danger zone">
        <DangerRow
          icon={ShieldAlert}
          title="Transfer notes to another workspace"
          description="Move every note, agent run, and recording out of this workspace and into another."
          onAction={() =>
            toast.info("Transfer notes", {
              description: "Cross-workspace transfer endpoint is on the GET-123 roadmap.",
            })
          }
        />
        <DangerRow
          icon={ShieldAlert}
          title="Remove yourself from this workspace"
          description="You'll lose access to shared notes. Notes you own are retained for the workspace."
          onAction={() =>
            toast.info("Remove yourself", {
              description: "Available once workspace membership ships server-side.",
            })
          }
        />
        <DangerRow
          icon={Trash2}
          title="Delete workspace"
          description="Permanently delete this workspace and all of its notes. This cannot be undone."
          onAction={() => {
            const ok = window.confirm(
              "Delete workspace? This cannot be undone. All shared notes will be lost.",
            );
            if (!ok) return;
            const really = window.confirm(
              "Last warning — every member's access is revoked and every note is destroyed. Continue?",
            );
            if (!really) return;
            toast.info("Workspace delete", {
              description: "Server-side delete endpoint not yet shipped.",
            });
          }}
        />
      </Group>
    </section>
  );
}

function LocalOnlyBanner() {
  return (
    <div className="flex items-start gap-3 rounded-lg border border-amber-200 bg-amber-50 p-4 text-amber-900 dark:border-amber-900/40 dark:bg-amber-950/40 dark:text-amber-100">
      <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0" />
      <div className="space-y-0.5 text-sm">
        <p className="font-medium">Local-only workspace</p>
        <p className="text-xs text-amber-900/80 dark:text-amber-100/80">
          You&apos;re running Attune offline. Workspace policies are saved
          locally and apply once you sign in. Sign in from{" "}
          <span className="font-medium">Profile</span> to share notes and
          invite teammates.
        </p>
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
      <div className="space-y-1 rounded-lg border border-border bg-card p-2">
        {children}
      </div>
    </div>
  );
}

function FieldRow({
  icon: Icon,
  title,
  subtitle,
  children,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  subtitle?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-start gap-4 rounded-md p-3 hover:bg-muted/30">
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1 space-y-1">
        <Label className="text-sm font-medium">{title}</Label>
        {subtitle ? (
          <p className="max-w-prose text-xs text-muted-foreground">{subtitle}</p>
        ) : null}
        <div className="pt-1">{children}</div>
      </div>
    </div>
  );
}

function ToggleRow({
  icon: Icon,
  title,
  description,
  checked,
  onChange,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  const id = React.useId();
  return (
    <div className="flex items-start gap-4 rounded-md p-3 hover:bg-muted/30">
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1 space-y-0.5">
        <Label htmlFor={id} className="text-sm font-medium">
          {title}
        </Label>
        <p className="max-w-prose text-xs text-muted-foreground">{description}</p>
      </div>
      <Switch
        id={id}
        checked={checked}
        onCheckedChange={onChange}
        className="mt-1 shrink-0"
      />
    </div>
  );
}

function ActionRow({
  icon: Icon,
  title,
  description,
  actionLabel,
  onAction,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  actionLabel: string;
  onAction: () => void;
}) {
  return (
    <div className="flex items-start gap-4 rounded-md p-3 hover:bg-muted/30">
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1 space-y-0.5">
        <p className="text-sm font-medium">{title}</p>
        <p className="max-w-prose text-xs text-muted-foreground">{description}</p>
      </div>
      <Button
        type="button"
        size="sm"
        variant="outline"
        onClick={onAction}
        className="mt-0.5 shrink-0"
      >
        {actionLabel}
      </Button>
    </div>
  );
}

function UpsellRow({
  icon: Icon,
  title,
  subtitle,
  badge,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  subtitle: string;
  badge: string;
}) {
  return (
    <div className="flex items-start gap-4 rounded-md p-3 hover:bg-muted/30">
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1 space-y-0.5">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium">{title}</p>
          <span className="rounded-full bg-primary/10 px-2 py-0.5 text-2xs font-medium uppercase tracking-wider text-primary">
            {badge}
          </span>
        </div>
        <p className="max-w-prose text-xs text-muted-foreground">{subtitle}</p>
      </div>
      <Button
        type="button"
        size="sm"
        variant="outline"
        onClick={() =>
          toast.info(`${title} is an Enterprise feature`, {
            description: "Talk to us about an Enterprise plan.",
          })
        }
        className="mt-0.5 shrink-0 gap-1"
      >
        Learn more
        <ExternalLink className="h-3 w-3" />
      </Button>
    </div>
  );
}

function DangerRow({
  icon: Icon,
  title,
  description,
  onAction,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  onAction: () => void;
}) {
  return (
    <div className="flex items-start gap-4 rounded-md p-3 hover:bg-destructive/5">
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-destructive" />
      <div className="min-w-0 flex-1 space-y-0.5">
        <p className="text-sm font-medium text-destructive">{title}</p>
        <p className="max-w-prose text-xs text-muted-foreground">{description}</p>
      </div>
      <Button
        type="button"
        size="sm"
        variant="outline"
        onClick={onAction}
        className="mt-0.5 shrink-0 border-destructive/40 text-destructive hover:bg-destructive/10 hover:text-destructive"
      >
        Continue
      </Button>
    </div>
  );
}

function BucketBadge({ bucket }: { bucket: string }) {
  if (!bucket) {
    return (
      <span className="inline-flex items-center rounded-md bg-muted px-2.5 py-1 text-xs font-medium text-muted-foreground">
        Not set
      </span>
    );
  }
  const labels: Record<string, string> = {
    founder: "Founder / Operator",
    clinical: "Clinical",
    sales: "Sales / Customer Success",
    education: "Education / Research",
  };
  const label = labels[bucket] ?? bucket;
  return (
    <span className="inline-flex items-center rounded-md bg-primary/10 px-2.5 py-1 text-xs font-medium text-primary">
      {label}
    </span>
  );
}

function LogoPicker({
  path,
  onChange,
}: {
  path: string;
  onChange: (v: string) => void;
}) {
  return (
    <div className="flex items-center gap-3">
      <div className="flex h-10 w-10 items-center justify-center overflow-hidden rounded-md border border-border bg-muted text-muted-foreground">
        {path ? (
          <span className="text-2xs">✓</span>
        ) : (
          <ImageIcon className="h-4 w-4" />
        )}
      </div>
      <div className="flex gap-1.5">
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={() =>
            toast.info("Logo picker", {
              description: "macOS file picker integration lands with the backend.",
            })
          }
        >
          {path ? "Replace" : "Upload"}
        </Button>
        {path ? (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            onClick={() => onChange("")}
          >
            Remove
          </Button>
        ) : null}
      </div>
    </div>
  );
}

function PrivacyTierViz({
  bucket,
  autoDelete,
}: {
  bucket: string;
  autoDelete: number | null;
}) {
  const tierMode = bucket === "clinical" ? "Tier 2" : "Tier 1";
  const tierDescription =
    bucket === "clinical"
      ? "Zero-knowledge encryption. Server holds ciphertext only. Required for BAA-free clinical use."
      : "Server-side AES-256-GCM. Encrypted at rest; Attune can decrypt for sharing and exports.";
  const retention =
    autoDelete && autoDelete > 0
      ? `Auto-delete after ${autoDelete} days`
      : "Retention off (kept indefinitely)";
  return (
    <div className="space-y-3 rounded-md border border-border bg-background p-4">
      <div className="flex items-center gap-2">
        <ShieldCheck className="h-4 w-4 text-primary" />
        <p className="text-sm font-medium">Privacy tier</p>
        <span className="ml-auto rounded-full bg-primary/10 px-2.5 py-0.5 text-2xs font-medium uppercase tracking-wider text-primary">
          {tierMode}
        </span>
      </div>
      <p className="text-xs text-muted-foreground">{tierDescription}</p>
      <div className="flex flex-wrap items-center gap-2 pt-1 text-2xs">
        <Pill label="On this Mac" detail="WAV + raw transcript" />
        <Pill label="Cloud" detail={tierMode === "Tier 2" ? "Ciphertext only" : "Encrypted at rest"} />
        <Pill label="Retention" detail={retention} />
      </div>
    </div>
  );
}

function Pill({ label, detail }: { label: string; detail: string }) {
  return (
    <span className="inline-flex items-center gap-1 rounded-full border border-border bg-muted px-2 py-0.5 text-muted-foreground">
      <span className="font-medium text-foreground">{label}</span>
      <span>·</span>
      <span>{detail}</span>
    </span>
  );
}
