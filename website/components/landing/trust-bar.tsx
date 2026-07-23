import { Cpu, GitBranch, MicOff, WifiOff } from "lucide-react";

const signals = [
    { icon: Cpu, label: "On-device transcription", detail: "Metal-accelerated Whisper" },
    { icon: WifiOff, label: "Works fully offline", detail: "Privacy Mode airgap" },
    { icon: MicOff, label: "No bots in your calls", detail: "System audio capture" },
    { icon: GitBranch, label: "Open source", detail: "Apache-2.0 licensed" },
];

export function TrustBar() {
    return (
        <section className="border-y border-border bg-secondary/40">
            <div className="container grid grid-cols-2 gap-px overflow-hidden lg:grid-cols-4">
                {signals.map((item) => (
                    <div
                        key={item.label}
                        className="flex flex-col gap-2 bg-background/0 px-2 py-8 sm:items-center sm:text-center"
                    >
                        <item.icon className="h-5 w-5 text-primary" />
                        <p className="text-ms-15 font-medium leading-tight">
                            {item.label}
                        </p>
                        <p className="text-2xs text-muted-foreground">{item.detail}</p>
                    </div>
                ))}
            </div>
        </section>
    );
}
