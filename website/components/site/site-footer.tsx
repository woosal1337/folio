import Link from "next/link";
import { Github } from "lucide-react";

import { footerNav, siteConfig } from "@/lib/site-config";
import { Logo } from "@/components/site/logo";

export function SiteFooter() {
    return (
        <footer className="border-t border-border bg-secondary/40">
            <div className="container py-16">
                <div className="grid gap-12 lg:grid-cols-[1.4fr_2fr]">
                    <div className="flex flex-col gap-4">
                        <Logo />
                        <p className="max-w-xs text-ms-15 leading-relaxed text-muted-foreground">
                            {siteConfig.description}
                        </p>
                        <Link
                            href={siteConfig.links.github}
                            target="_blank"
                            rel="noreferrer"
                            className="focus-ring inline-flex w-fit items-center gap-2 rounded-md text-sm text-muted-foreground transition-colors hover:text-foreground"
                        >
                            <Github className="h-4 w-4" />
                            github.com/woosal1337/folio
                        </Link>
                    </div>

                    <div className="grid grid-cols-2 gap-8 sm:grid-cols-3">
                        {footerNav.map((group) => (
                            <div key={group.title} className="flex flex-col gap-3">
                                <p className="font-mono text-2xs uppercase tracking-[0.16em] text-muted-foreground">
                                    {group.title}
                                </p>
                                <ul className="flex flex-col gap-2.5">
                                    {group.items.map((item) => (
                                        <li key={item.href}>
                                            <Link
                                                href={item.href}
                                                target={
                                                    item.external ? "_blank" : undefined
                                                }
                                                rel={
                                                    item.external
                                                        ? "noreferrer"
                                                        : undefined
                                                }
                                                className="text-ms-15 text-muted-foreground transition-colors hover:text-foreground"
                                            >
                                                {item.label}
                                            </Link>
                                        </li>
                                    ))}
                                </ul>
                            </div>
                        ))}
                    </div>
                </div>

                <div className="mt-14 flex flex-col gap-4 border-t border-border pt-8 text-ms-13 text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
                    <p>
                        {siteConfig.license} licensed. Built for {siteConfig.platform}.
                        Audio stays on your machine.
                    </p>
                    <p className="font-mono text-2xs uppercase tracking-[0.14em]">
                        Folio v{siteConfig.version}
                    </p>
                </div>
            </div>
        </footer>
    );
}
