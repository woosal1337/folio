import { Label } from "@/shared/ui/label";
import { Separator } from "@/shared/ui/separator";
import { Switch } from "@/shared/ui/switch";
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
    </div>
  );
}
