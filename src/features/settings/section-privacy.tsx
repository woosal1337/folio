import { Lock, ShieldCheck } from "lucide-react";

import { Switch } from "@/shared/ui/switch";
import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

export function SectionPrivacy({ settings, onChange }: Props) {
  return (
    <div className="flex flex-col gap-7">
      <div>
        <h2 className="font-serif text-2xl font-medium">Privacy</h2>
        <p className="mt-1 text-sm text-muted-foreground">
          Everything Folio does stays on your Mac unless you opt in here. Toggles
          default OFF.
        </p>
      </div>

      <section
        aria-label="Privacy Mode"
        className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4"
      >
        <div className="flex items-start gap-3">
          <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0 text-emerald-600 dark:text-emerald-400" />
          <div className="flex-1">
            <p className="text-sm font-medium">Privacy Mode (Airgap)</p>
            <p className="mt-0.5 text-xs text-muted-foreground">
              Physically blocks every outbound network call: LLM providers, embedding
              APIs, model downloads, webhooks. Local services on{" "}
              <code className="rounded bg-muted px-1 py-0.5 text-2xs">localhost</code>{" "}
              keep working, so a local Whisper + a local LLM let the app run end-to-end
              with wifi off. The titlebar shows an{" "}
              <span className="rounded-full bg-emerald-500/10 px-1.5 py-0.5 text-2xs font-medium uppercase tracking-wider text-emerald-700 ring-1 ring-emerald-500/30 dark:text-emerald-300">
                Airgap
              </span>{" "}
              badge while this is on.
            </p>
          </div>
          <Switch
            checked={settings.privacy_mode}
            onCheckedChange={(v) => onChange("privacy_mode", v)}
            className="mt-1"
            aria-label="Privacy Mode"
          />
        </div>
      </section>

      <section
        aria-label="Anonymous aggregate stats"
        className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4"
      >
        <div className="flex items-start gap-3">
          <Lock className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="flex-1">
            <p className="text-sm font-medium">
              Contribute to the public aggregate counter
            </p>
            <p className="mt-0.5 text-xs text-muted-foreground">
              When on, Folio adds three numbers to a public counter at{" "}
              <code className="rounded bg-muted px-1 py-0.5 text-2xs">
                folio.app/stats
              </code>
              : total minutes transcribed locally, total cost saved over cloud Whisper,
              and a still-alive install ping. No content and no identifiers ever leave
              your machine — the upload is three integers and an opaque per-install
              salt. Designed as a public trust artifact for the BYO-key audience.
            </p>
          </div>
          <Switch
            checked={settings.share_aggregate_stats}
            onCheckedChange={(v) => onChange("share_aggregate_stats", v)}
            className="mt-1"
            aria-label="Share anonymous aggregate stats"
          />
        </div>
      </section>
    </div>
  );
}
