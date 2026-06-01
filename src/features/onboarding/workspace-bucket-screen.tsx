/**
 * GET-129 — Workspace segmentation.
 *
 * Four cards: Founder / Healthcare / Sales / Education. The bucket
 * is a free-text label used to tune templates and terminology in
 * summaries. It has no privacy, encryption, or verification side
 * effects — Attune is a general transcription app, and any
 * vertical-specific gating (e.g. clinical license verification)
 * has been deferred (see Linear GET-130).
 *
 * Tony: 2x2 SF-Symbol-style grid, equal-weight cards. No "primary"
 * suggestion — the question is genuinely about the user, not a
 * sales funnel.
 */

import * as React from "react";
import { Rocket, Stethoscope, TrendingUp, GraduationCap } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { cn } from "@/shared/lib/utils";

type Bucket = "founder" | "healthcare" | "sales" | "education";

interface BucketCard {
  id: Bucket;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  tagline: string;
}

const BUCKETS: BucketCard[] = [
  {
    id: "founder",
    icon: Rocket,
    label: "Founder / Operator",
    tagline: "Investor calls, hiring, customer interviews.",
  },
  {
    id: "healthcare",
    icon: Stethoscope,
    label: "Healthcare",
    tagline: "Patient consults, therapy notes, medical interviews.",
  },
  {
    id: "sales",
    icon: TrendingUp,
    label: "Sales / Customer Success",
    tagline: "Discovery, demos, QBRs synced to your CRM.",
  },
  {
    id: "education",
    icon: GraduationCap,
    label: "Education / Research",
    tagline: "Lectures, interviews, fieldwork transcripts.",
  },
];

interface Props {
  initial?: Bucket | "";
  onContinue: (bucket: Bucket) => void | Promise<void>;
}

export function WorkspaceBucketScreen({ initial, onContinue }: Props) {
  const [selected, setSelected] = React.useState<Bucket | null>(
    initial ? (initial as Bucket) : null
  );
  const [submitting, setSubmitting] = React.useState(false);

  const submit = async () => {
    if (!selected || submitting) return;
    setSubmitting(true);
    try {
      await onContinue(selected);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-7 px-6 py-12">
      <header data-drag="" className="select-none">
        <h1 className="font-serif text-3xl font-medium tracking-tight">
          What do you do?
        </h1>
        <p className="mt-2 max-w-prose text-sm text-muted-foreground">
          We&apos;ll tune templates, terminology, and privacy defaults to fit. You can
          change this later in Settings.
        </p>
      </header>

      <div
        role="radiogroup"
        aria-label="Workspace segmentation"
        className="grid grid-cols-1 gap-3 sm:grid-cols-2"
      >
        {BUCKETS.map((b) => (
          <BucketTile
            key={b.id}
            card={b}
            selected={selected === b.id}
            onSelect={() => setSelected(b.id)}
          />
        ))}
      </div>

      <Button
        size="lg"
        onClick={submit}
        disabled={!selected || submitting}
        className="h-11"
      >
        {submitting ? "Saving…" : "Continue"}
      </Button>
    </div>
  );
}

function BucketTile({
  card,
  selected,
  onSelect,
}: {
  card: BucketCard;
  selected: boolean;
  onSelect: () => void;
}) {
  const Icon = card.icon;
  return (
    <button
      type="button"
      role="radio"
      aria-checked={selected}
      onClick={onSelect}
      className={cn(
        "group relative flex flex-col items-start gap-2 rounded-lg border bg-card p-5 text-left transition-colors",
        "hover:border-primary/40 hover:bg-muted/30",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        selected
          ? "border-primary bg-primary/[0.04] ring-1 ring-primary/40"
          : "border-border"
      )}
    >
      <div
        className={cn(
          "flex h-9 w-9 items-center justify-center rounded-md transition-colors",
          selected ? "bg-primary/10 text-primary" : "bg-muted text-muted-foreground"
        )}
      >
        <Icon className="h-4.5 w-4.5" />
      </div>
      <div className="mt-1">
        <p className="text-sm font-medium">{card.label}</p>
        <p className="mt-0.5 text-xs text-muted-foreground">{card.tagline}</p>
      </div>
    </button>
  );
}
