import { Check, Minus } from "lucide-react";

import { cn } from "@/lib/utils";

type Row = {
    capability: string;
    local: boolean;
    detail: string;
};

const rows: Row[] = [
    { capability: "Audio capture", local: true, detail: "cpal + ScreenCaptureKit" },
    {
        capability: "Transcription",
        local: true,
        detail: "whisper.cpp, Metal-accelerated",
    },
    { capability: "Speaker diarization", local: true, detail: "pyannote + sherpa-onnx" },
    { capability: "Note storage", local: true, detail: "Markdown in your vault" },
    { capability: "Encryption at rest", local: true, detail: "AES-256-GCM + Argon2id" },
    { capability: "Connectors (MCP)", local: true, detail: "stdio, read-only" },
    { capability: "Cloud transcription", local: false, detail: "Opt-in OpenAI Whisper" },
    {
        capability: "Chat & webhooks",
        local: false,
        detail: "Opt-in, Privacy Mode blocks",
    },
];

export function CapabilityTable() {
    return (
        <div className="overflow-hidden rounded-2xl border border-border bg-card shadow-sm">
            <div className="grid grid-cols-[1fr_auto_1.1fr] items-center gap-4 border-b border-border bg-secondary/50 px-6 py-3 font-mono text-2xs uppercase tracking-[0.16em] text-muted-foreground">
                <span>Capability</span>
                <span className="text-center">Runs</span>
                <span>Backend</span>
            </div>
            <ul className="divide-y divide-border">
                {rows.map((row) => (
                    <li
                        key={row.capability}
                        className="grid grid-cols-[1fr_auto_1.1fr] items-center gap-4 px-6 py-4"
                    >
                        <span className="text-ms-15 font-medium">{row.capability}</span>
                        <span
                            className={cn(
                                "inline-flex items-center gap-1.5 justify-self-center rounded-full px-2.5 py-0.5 font-mono text-2xs",
                                row.local
                                    ? "bg-primary/10 text-primary"
                                    : "bg-muted text-muted-foreground"
                            )}
                        >
                            {row.local ? (
                                <>
                                    <Check className="h-3 w-3" />
                                    local
                                </>
                            ) : (
                                <>
                                    <Minus className="h-3 w-3" />
                                    opt-in
                                </>
                            )}
                        </span>
                        <span className="text-ms-13 text-muted-foreground">
                            {row.detail}
                        </span>
                    </li>
                ))}
            </ul>
        </div>
    );
}
