import { Mic, Volume2 } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import { Separator } from "@/shared/ui/separator";
import { Switch } from "@/shared/ui/switch";
import { playFeedback } from "@/shared/lib/feedback";
import type { DeviceInfo } from "@/shared/types/DeviceInfo";
import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
  devices: DeviceInfo[];
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

export function SectionGeneral({ settings, devices, onChange }: Props) {
  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">General</h2>

      <section className="space-y-3">
        <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Microphone
        </Label>
        <select
          value={settings.mic_device ?? ""}
          onChange={(e) =>
            onChange("mic_device", e.target.value === "" ? null : e.target.value)
          }
          className="h-9 w-full rounded-md border border-input bg-card px-3 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <option value="">(system default)</option>
          {devices.map((d) => (
            <option key={d.name} value={d.name}>
              {d.name}
              {d.is_default ? " · default" : ""}
            </option>
          ))}
        </select>
        <p className="text-xs text-muted-foreground">
          {devices.length} input device{devices.length === 1 ? "" : "s"} visible to the
          system.
        </p>
      </section>

      <Separator />

      <section className="flex items-start justify-between gap-6">
        <div>
          <h3 className="font-medium">Capture system audio</h3>
          <p className="mt-1 max-w-md text-xs text-muted-foreground">
            Records what comes out of your speakers via ScreenCaptureKit. macOS prompts
            for Screen Recording permission the first time. Audio only — no video.
          </p>
        </div>
        <Switch
          checked={settings.system_audio_enabled}
          onCheckedChange={(v) => onChange("system_audio_enabled", v)}
        />
      </section>

      <Separator />

      <section className="space-y-3">
        <div className="flex items-start justify-between gap-6">
          <div>
            <h3 className="flex items-center gap-2 font-medium">
              <Volume2 className="h-4 w-4 text-muted-foreground" />
              Feedback sounds
            </h3>
            <p className="mt-1 max-w-md text-xs text-muted-foreground">
              Tiny synthesised tones on recording start / stop and when an agent
              finishes. Off by default. Suppressed when the OS reports Reduce Motion.
            </p>
          </div>
          <Switch
            checked={settings.feedback_sounds_enabled}
            onCheckedChange={(v) => onChange("feedback_sounds_enabled", v)}
          />
        </div>
        {settings.feedback_sounds_enabled ? (
          <div className="flex flex-wrap items-center gap-1.5 pt-1">
            <span className="text-2xs uppercase tracking-wider text-muted-foreground">
              Preview
            </span>
            {(["start", "stop", "success", "dismiss", "error"] as const).map((k) => (
              <Button
                key={k}
                type="button"
                size="sm"
                variant="ghost"
                className="h-6 px-2 text-2xs"
                onClick={() => playFeedback(k)}
              >
                {k}
              </Button>
            ))}
          </div>
        ) : null}
      </section>

      <Separator />

      <section className="space-y-3">
        <div className="flex items-start justify-between gap-6">
          <div>
            <h3 className="flex items-center gap-2 font-medium">
              <Mic className="h-4 w-4 text-muted-foreground" />
              Voice debrief on Stop
            </h3>
            <p className="mt-1 max-w-md text-xs text-muted-foreground">
              After you hit Stop, Folio asks &ldquo;anything to capture before this
              fades?&rdquo; and records up to 20s of mic. The clip rides into the same
              extract-tasks / extract-memories pass as the meeting. Off by default.
            </p>
          </div>
          <Switch
            checked={settings.voice_debrief_enabled}
            onCheckedChange={(v) => onChange("voice_debrief_enabled", v)}
          />
        </div>
      </section>
    </div>
  );
}
