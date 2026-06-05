import * as React from "react";
import { Headphones, Mic, Square, Volume2 } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import { listInputDevices, startMicMonitor, stopMicMonitor } from "@/shared/lib/ipc";
import type { DeviceInfo } from "@/shared/types/DeviceInfo";
import type { Settings } from "@/shared/types/Settings";

interface Props {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

export function SectionAudio({ settings, onChange }: Props) {
  const [devices, setDevices] = React.useState<DeviceInfo[]>([]);
  React.useEffect(() => {
    listInputDevices()
      .then(setDevices)
      .catch(() => {});
  }, []);

  const [monitoring, setMonitoring] = React.useState(false);
  const [monitorError, setMonitorError] = React.useState<string | null>(null);

  React.useEffect(() => {
    return () => {
      if (monitoring) {
        void stopMicMonitor().catch(() => {});
      }
    };
  }, [monitoring]);

  const toggleMonitor = async () => {
    setMonitorError(null);
    if (monitoring) {
      await stopMicMonitor().catch((e) => setMonitorError(String(e)));
      setMonitoring(false);
    } else {
      try {
        await startMicMonitor(settings.mic_device ?? undefined);
        setMonitoring(true);
      } catch (e) {
        setMonitorError(String(e));
      }
    }
  };

  return (
    <div className="flex flex-col gap-7">
      <h2 className="font-serif text-2xl font-medium">Audio</h2>

      <section className="space-y-3">
        <Label className="flex items-center gap-2 text-sm font-medium">
          <Mic className="h-4 w-4 text-muted-foreground" />
          Microphone
        </Label>

        <div className="flex items-center gap-3">
          <select
            value={settings.mic_device ?? ""}
            onChange={(e) => onChange("mic_device", e.target.value || null)}
            className="h-9 flex-1 rounded-md border border-input bg-card px-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            aria-label="Select microphone"
          >
            <option value="">System default</option>
            {devices.map((d) => (
              <option key={d.name} value={d.name}>
                {d.name}
                {d.is_default ? " (default)" : ""}
              </option>
            ))}
          </select>

          <Button
            type="button"
            variant={monitoring ? "destructive" : "outline"}
            size="sm"
            className="shrink-0 gap-2"
            onClick={() => void toggleMonitor()}
          >
            {monitoring ? (
              <>
                <Square className="h-3.5 w-3.5 fill-current" />
                Stop test
              </>
            ) : (
              <>
                <Volume2 className="h-3.5 w-3.5" />
                Test mic
              </>
            )}
          </Button>
        </div>

        {monitoring ? (
          <p className="text-xs text-primary">
            🎙 Listening — you should hear your mic through your speakers or headphones.
            Click <strong>Stop test</strong> when done.
          </p>
        ) : (
          <p className="text-xs text-muted-foreground">
            Click <strong>Test mic</strong> to hear your microphone played back through
            your output — adjust input volume in System Settings → Sound → Input until
            it sounds clear.
          </p>
        )}

        {monitorError ? (
          <p className="text-xs text-destructive">{monitorError}</p>
        ) : null}
      </section>

      <section className="space-y-4">
        <div className="flex items-start justify-between gap-6">
          <div className="space-y-1">
            <Label
              htmlFor="voice-processing-toggle"
              className="flex items-center gap-2 text-sm font-medium"
            >
              <Headphones className="h-4 w-4 text-muted-foreground" />
              Voice processing
            </Label>
            <p className="max-w-md text-xs text-muted-foreground">
              Routes the mic through Apple&apos;s Voice Processing IO AudioUnit —
              acoustic echo cancellation, noise suppression, and automatic gain control.
              Stops the mic from picking up system audio when you are not wearing
              headphones. Same technology Zoom, FaceTime, and Discord use on macOS.{" "}
              <strong className="text-foreground">
                Leave this off if your mic records nothing
              </strong>{" "}
              — on some Macs this path captures silence; plain capture is the reliable
              default.
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
