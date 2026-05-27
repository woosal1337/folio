/**
 * GET-140 — Settings → Referrals.
 *
 * Tony's "half-page" layout: hero, personal link, three-step
 * explainer, three rules. The personal token comes from the
 * /referrals/generate endpoint when the backend ships; until then
 * we display a placeholder token derived from the local user so the
 * copy + email-share affordances are exercisable.
 */

import * as React from "react";
import { Check, Copy, Gift, Mail } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";

const STUB_TOKEN_PLACEHOLDER = "join-attune-locally";

export function SectionReferrals() {
  const [copied, setCopied] = React.useState(false);

  const link = `https://join.attune.app/t/${STUB_TOKEN_PLACEHOLDER}`;
  const subject = "Try Attune — your meetings stay on your Mac";
  const body = `Hey,

I've been using Attune for meeting notes — it runs locally on your Mac so transcripts never leave the device. Thought you might like it.

If you sign up with my link you get 2 months of Attune Pro free:

${link}

— sent from Attune`;

  const mailto = `mailto:?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;

  const copyLink = async () => {
    try {
      await navigator.clipboard.writeText(link);
      setCopied(true);
      toast.success("Referral link copied");
      setTimeout(() => setCopied(false), 2000);
    } catch (e) {
      console.error("clipboard:", e);
      toast.error("Could not copy link", { description: String(e) });
    }
  };

  return (
    <section className="space-y-7">
      <header className="space-y-2">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <Gift className="h-5 w-5" />
        </div>
        <h2 className="font-serif text-2xl font-medium">
          Know someone who&apos;d love their meetings to stay on their Mac?
        </h2>
        <p className="text-sm text-muted-foreground">
          Give them 2 months of Attune Pro free.
        </p>
      </header>

      <div className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Your referral link
        </Label>
        <div className="flex items-center gap-2 rounded-lg border border-border bg-card p-2 pl-3">
          <code className="flex-1 truncate font-mono text-xs text-foreground">{link}</code>
          <Button type="button" size="sm" variant="outline" onClick={copyLink} className="gap-1.5">
            {copied ? <Check className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
            {copied ? "Copied" : "Copy"}
          </Button>
          <Button asChild type="button" size="sm" className="gap-1.5">
            <a href={mailto}>
              <Mail className="h-3.5 w-3.5" />
              Email
            </a>
          </Button>
        </div>
        <p className="text-2xs text-muted-foreground">
          Your personal token activates when the backend ships. Until then,
          this link demonstrates the flow.
        </p>
      </div>

      <div className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          How it works
        </Label>
        <ol className="space-y-2 rounded-lg border border-border bg-card p-5">
          <Step n={1}>Share your link.</Step>
          <Step n={2}>They sign up with a work email.</Step>
          <Step n={3}>They unlock 2 months of Attune Pro on us — and so do you.</Step>
        </ol>
      </div>

      <div className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          A few rules
        </Label>
        <ul className="space-y-2 rounded-lg border border-border bg-card p-5 text-sm text-muted-foreground">
          <Rule>They must be new to Attune (no existing account).</Rule>
          <Rule>Their company doesn&apos;t already have an Attune workspace.</Rule>
          <Rule>They sign up with a work email — personal Gmail / Outlook are not eligible.</Rule>
        </ul>
      </div>
    </section>
  );
}

function Step({ n, children }: { n: number; children: React.ReactNode }) {
  return (
    <li className="flex items-start gap-3">
      <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary/10 text-2xs font-medium text-primary">
        {n}
      </span>
      <span className="text-sm text-foreground">{children}</span>
    </li>
  );
}

function Rule({ children }: { children: React.ReactNode }) {
  return (
    <li className="flex items-start gap-3 text-sm">
      <span className="mt-2 h-1 w-1 shrink-0 rounded-full bg-muted-foreground" />
      <span>{children}</span>
    </li>
  );
}
