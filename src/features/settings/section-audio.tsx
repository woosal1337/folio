import { Headphones, Mic } from "lucide-react";

import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

/**
 * Audio capture settings. Phase 2 of the AI chat plan adds the
 * Voice Processing IO toggle here — it's the only knob today,
 * future audio knobs (input device override, gain control, AEC
 * tiers) will land in this same section.
 */
export function SectionAudio({ settings, onChange }: Props) {
  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Audio</h2>

      <section className="space-y-4">
        <div className="flex items-start justify-between gap-6">
          <div className="space-y-1">
            <Label
              htmlFor="voice-processing-toggle"
              className="flex items-center gap-2 text-sm font-medium"
            >
              <Mic className="h-4 w-4 text-muted-foreground" />
              Voice processing
              <span className="rounded bg-muted px-1.5 py-0.5 text-2xs font-normal text-muted-foreground">
                Recommended
              </span>
            </Label>
            <p className="max-w-md text-xs text-muted-foreground">
              Routes the mic through Apple&apos;s Voice Processing IO AudioUnit —
              acoustic echo cancellation, noise suppression, and automatic gain control.
              Stops the mic from picking up system audio when you are not wearing
              headphones. Same technology Zoom, FaceTime, and Discord use on macOS.
              Falls back to plain capture automatically if it fails to initialise on
              your device.
            </p>
          </div>
          <Switch
            id="voice-processing-toggle"
            checked={settings.voice_processing_enabled}
            onCheckedChange={(checked) => onChange("voice_processing_enabled", checked)}
            className="mt-1"
          />
        </div>

        <div className="rounded-lg border border-border bg-muted/40 p-3 text-xs text-muted-foreground">
          <div className="mb-1 flex items-center gap-2 text-foreground">
            <Headphones className="h-3.5 w-3.5" />
            <span className="font-medium">When does this matter?</span>
          </div>
          Voice processing kicks in when audio is leaving the laptop speakers and the
          mic is picking it back up. With headphones plugged in there is no bleed to
          cancel and the only effect is the bundled noise suppression and AGC, which are
          still useful.
        </div>
      </section>
    </div>
  );
}
