import Link from "next/link";
import { ArrowRight } from "lucide-react";

import { siteConfig } from "@/lib/site-config";
import { Button } from "@/components/ui/button";
import { CommandLine } from "@/components/site/code-block";
import { FolioMark } from "@/components/site/logo";
import { WaveBars } from "@/components/landing/wave-bars";

export function CtaSection() {
    return (
        <section className="container py-20 sm:py-28">
            <div className="relative overflow-hidden rounded-2xl border border-border bg-card px-6 py-16 text-center shadow-lift sm:px-12">
                <div
                    className="pointer-events-none absolute inset-0 -z-10 opacity-80"
                    aria-hidden
                    style={{
                        background:
                            "radial-gradient(50% 60% at 50% 0%, hsl(var(--accent) / 0.7) 0%, transparent 70%)",
                    }}
                />
                <div className="mx-auto flex max-w-2xl flex-col items-center gap-6">
                    <FolioMark className="h-12 w-12" />
                    <h2 className="text-balance font-display text-ms-34 font-semibold tracking-tight sm:text-ms-45">
                        Own your meeting notes
                    </h2>
                    <p className="text-pretty text-ms-17 leading-relaxed text-muted-foreground">
                        Install {siteConfig.name}, grant microphone and screen-recording
                        permission, and your next meeting writes itself to your vault.
                    </p>
                    <WaveBars className="h-7 w-40 text-primary/60" />
                    <div className="flex flex-col gap-3 sm:flex-row">
                        <Button asChild size="lg">
                            <Link href="/docs/installation">
                                Install for {siteConfig.platform}
                                <ArrowRight className="h-4 w-4" />
                            </Link>
                        </Button>
                        <Button asChild size="lg" variant="outline">
                            <Link
                                href={siteConfig.links.github}
                                target="_blank"
                                rel="noreferrer"
                            >
                                Star on GitHub
                            </Link>
                        </Button>
                    </div>
                    <CommandLine
                        command={siteConfig.install.installCommand}
                        className="w-full max-w-sm"
                    />
                </div>
            </div>
        </section>
    );
}
