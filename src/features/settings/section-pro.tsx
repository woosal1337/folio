import * as React from "react";
import { Crown, ExternalLink } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

const PRO_FEATURES = [
  "Auto-record on calendar events",
  "Multi-window kanban + memory side-by-side",
  "Template marketplace install",
  "Lifetime updates inside the major version",
  "Priority issue triage on GitHub",
];

/**
 * Settings → Pro. Surfaces the user's current tier and the license
 * key field. Free tier shows the upgrade CTA + Pay-What-You-Want
 * slider (defaults to $79; user can move between $39 and $149).
 *
 * v2 roadmap finding 092 / GET-108. The buy link goes through the
 * opener plugin so the user lands in their default browser instead
 * of the embedded WebView.
 */
export function SectionPro({ settings, onChange }: Props) {
  const isPro = settings.pro_license_key.trim().length > 0;
  const [price, setPrice] = React.useState(79);

  const handleBuy = async () => {
    const url = `https://attune.app/buy?price=${price}`;
    try {
      await openUrl(url);
    } catch (e) {
      console.error("openUrl:", e);
      window.open(url, "_blank");
    }
  };

  return (
    <div className="flex flex-col gap-7">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="font-serif text-2xl font-medium">Attune Pro</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            One-time license. No subscription, no recurring charges, no account.
            Lifetime updates within the same major version. Buy once, own forever —
            Sketch / Bear / Things lineage.
          </p>
        </div>
        <span
          className={
            isPro
              ? "rounded-full border border-primary bg-accent px-3 py-1 text-xs font-medium tabular-nums text-accent-foreground"
              : "rounded-full border border-border bg-muted px-3 py-1 text-xs font-medium tabular-nums text-muted-foreground"
          }
        >
          {isPro ? "Pro" : "Free"}
        </span>
      </div>

      {isPro ? (
        <section className="rounded-lg border border-border bg-card p-4">
          <div className="flex items-center gap-2 text-sm">
            <Crown className="h-4 w-4 text-primary" />
            <span className="font-medium">Thanks for supporting Attune.</span>
          </div>
          <p className="mt-1 text-xs text-muted-foreground">
            Your license key is saved locally. To transfer to another Mac, copy the key
            below and paste it on the new install.
          </p>
          <Label className="mt-3 block text-xs">
            License key
            <Input
              value={settings.pro_license_key}
              onChange={(e) => onChange("pro_license_key", e.target.value)}
              className="mt-1 font-mono text-xs"
            />
          </Label>
        </section>
      ) : (
        <section className="flex flex-col gap-3 rounded-lg border border-border bg-card p-4">
          <p className="text-sm font-medium">Pro unlocks:</p>
          <ul className="ml-4 list-disc text-xs text-muted-foreground">
            {PRO_FEATURES.map((f) => (
              <li key={f}>{f}</li>
            ))}
          </ul>
          <div className="mt-2 flex flex-col gap-2">
            <Label className="text-xs">
              Pay what you want — ${price}
              <input
                type="range"
                min={39}
                max={149}
                step={5}
                value={price}
                onChange={(e) => setPrice(Number(e.target.value))}
                className="mt-1 w-full accent-primary"
                aria-label="Price slider"
              />
              <div className="mt-0.5 flex justify-between text-2xs tabular-nums text-muted-foreground">
                <span>$39</span>
                <span className="font-medium">$79 suggested</span>
                <span>$149</span>
              </div>
            </Label>
            <Button onClick={handleBuy} className="gap-2 self-start">
              <Crown className="h-3.5 w-3.5" />
              Buy Attune Pro · ${price}
              <ExternalLink className="h-3 w-3 opacity-60" />
            </Button>
            <p className="text-2xs text-muted-foreground">
              After purchase, paste the license key here to unlock Pro features.
            </p>
          </div>
          <div className="mt-2 flex flex-col gap-2 border-t border-border pt-3">
            <Label className="text-xs">
              I have a license key
              <Input
                value={settings.pro_license_key}
                onChange={(e) => onChange("pro_license_key", e.target.value.trim())}
                placeholder="ATTUNE-PRO-XXXX-XXXX-XXXX"
                className="mt-1 font-mono text-xs"
              />
            </Label>
          </div>
        </section>
      )}
    </div>
  );
}
