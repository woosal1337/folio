/**
 * GET-139 — Settings → Workspace → Billing.
 *
 * Tier badge, per-tier feature matrix, Stripe-hosted payment surface
 * (never touch card data ourselves), invoice history.
 *
 * v1 shows the tier matrix + a "Manage billing" deeplink stub. The
 * Stripe Checkout integration ships alongside the attune-api auth
 * surface; pricing is still pending (see GET-139 description).
 */

import * as React from "react";
import { CheckCircle2, Receipt, Shield, Sparkles, Zap } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";

type Tier = "free" | "pro" | "team" | "enterprise";

const TIER_LABEL: Record<Tier, string> = {
  free: "Free",
  pro: "Pro",
  team: "Team",
  enterprise: "Enterprise",
};

interface FeatureMatrixRow {
  feature: string;
  free: string | boolean;
  pro: string | boolean;
  team: string | boolean;
  enterprise: string | boolean;
}

const MATRIX: FeatureMatrixRow[] = [
  { feature: "Unlimited local recording", free: true, pro: true, team: true, enterprise: true },
  { feature: "Local Whisper transcription", free: true, pro: true, team: true, enterprise: true },
  { feature: "Cloud transcription (OpenAI / others)", free: "Bring your own key", pro: true, team: true, enterprise: true },
  { feature: "AI agents (summarise, tasks, memory)", free: "5/mo", pro: "Unlimited", team: "Unlimited", enterprise: "Unlimited" },
  { feature: "Shared notes across workspace", free: false, pro: false, team: true, enterprise: true },
  { feature: "MCP server (local)", free: true, pro: true, team: true, enterprise: true },
  { feature: "Auto-record on calendar match", free: false, pro: true, team: true, enterprise: true },
  { feature: "Custom AI agents", free: false, pro: true, team: true, enterprise: true },
  { feature: "SSO + SCIM", free: false, pro: false, team: false, enterprise: true },
  { feature: "Workspace audit log", free: false, pro: false, team: true, enterprise: true },
];

export function SectionWorkspaceBilling() {
  // v1: tier is Free until the licensing backend lands. Once the
  // pro_license_key field is populated (existing) or the workspace
  // tier ships server-side, this resolves to the real tier.
  const tier: Tier = "free";

  const openStripe = () => {
    toast.info("Billing", {
      description:
        "Stripe-hosted checkout lands with attune-api. Card data is never stored by Attune.",
    });
  };

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Billing</h2>
        <p className="text-sm text-muted-foreground">
          Tier, payment, and invoice history for your workspace.
        </p>
      </header>

      <CurrentTierCard tier={tier} onManage={openStripe} />

      <Group title="What each tier includes">
        <FeatureMatrix current={tier} />
      </Group>

      <Group title="Invoices">
        <div className="flex items-start gap-3 rounded-lg border border-dashed border-border bg-card p-5">
          <Receipt className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1 space-y-0.5">
            <p className="text-sm font-medium">No invoices yet</p>
            <p className="max-w-prose text-xs text-muted-foreground">
              Past invoices appear here once you upgrade to a paid plan.
              Downloads are served by Stripe — Attune never sees the payment
              data itself.
            </p>
          </div>
        </div>
      </Group>

      <SecurityNote />
    </section>
  );
}

function CurrentTierCard({ tier, onManage }: { tier: Tier; onManage: () => void }) {
  const Icon = tier === "free" ? Sparkles : tier === "enterprise" ? Shield : Zap;
  return (
    <div className="rounded-lg border border-border bg-card p-5">
      <div className="flex flex-wrap items-start gap-3">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary">
          <Icon className="h-5 w-5" />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <p className="text-2xs uppercase tracking-wider text-muted-foreground">
              Current tier
            </p>
          </div>
          <p className="mt-0.5 font-serif text-2xl font-medium">
            {TIER_LABEL[tier]}
          </p>
          {tier === "free" ? (
            <p className="mt-1 text-xs text-muted-foreground">
              All core recording + local transcription features. Upgrade to
              unlock cloud transcription, shared notes, and custom agents.
            </p>
          ) : (
            <p className="mt-1 text-xs text-muted-foreground">
              Manage payment, change plan, or cancel anytime.
            </p>
          )}
        </div>
        <Button type="button" onClick={onManage} className="shrink-0">
          {tier === "free" ? "Upgrade" : "Manage"}
        </Button>
      </div>
    </div>
  );
}

function FeatureMatrix({ current }: { current: Tier }) {
  const tiers: Tier[] = ["free", "pro", "team", "enterprise"];
  return (
    <div className="overflow-hidden rounded-lg border border-border bg-card">
      <div className="grid grid-cols-[1fr_repeat(4,minmax(0,90px))] border-b border-border bg-muted/40 text-2xs font-medium uppercase tracking-wider text-muted-foreground">
        <div className="px-4 py-3">Feature</div>
        {tiers.map((t) => (
          <div
            key={t}
            className={`px-3 py-3 text-center ${current === t ? "text-primary" : ""}`}
          >
            {TIER_LABEL[t]}
          </div>
        ))}
      </div>
      {MATRIX.map((row, i) => (
        <div
          key={row.feature}
          className={`grid grid-cols-[1fr_repeat(4,minmax(0,90px))] text-xs ${
            i % 2 ? "bg-muted/20" : ""
          }`}
        >
          <div className="px-4 py-2.5 text-muted-foreground">{row.feature}</div>
          {tiers.map((t) => (
            <div
              key={t}
              className={`flex items-center justify-center px-3 py-2.5 ${
                current === t ? "bg-primary/5" : ""
              }`}
            >
              <Cell value={row[t]} />
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

function Cell({ value }: { value: string | boolean }) {
  if (value === true) {
    return <CheckCircle2 className="h-3.5 w-3.5 text-primary" />;
  }
  if (value === false) {
    return <span className="text-muted-foreground/40">—</span>;
  }
  return <span className="text-center text-2xs text-muted-foreground">{value}</span>;
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

function SecurityNote() {
  return (
    <div className="flex items-start gap-3 rounded-lg border border-border bg-muted/30 p-4">
      <Shield className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <p className="flex-1 text-xs text-muted-foreground">
        Payments are processed by Stripe. Attune never stores or sees your
        card number, expiry, or CVV — only the last four digits + brand for
        display.
      </p>
    </div>
  );
}
