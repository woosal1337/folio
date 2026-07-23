import Link from "next/link";
import { Github } from "lucide-react";

import { mainNav, siteConfig } from "@/lib/site-config";
import { Button } from "@/components/ui/button";
import { Logo } from "@/components/site/logo";
import { MobileNav } from "@/components/site/mobile-nav";

export function SiteHeader() {
    return (
        <header className="sticky top-0 z-40 border-b border-border/70 bg-background/80 backdrop-blur-md">
            <div className="container flex h-16 items-center justify-between gap-6">
                <Link
                    href="/"
                    className="focus-ring rounded-md"
                    aria-label={siteConfig.name}
                >
                    <Logo />
                </Link>

                <nav className="hidden items-center gap-1 md:flex">
                    {mainNav.map((item) => (
                        <Link
                            key={item.href}
                            href={item.href}
                            target={item.external ? "_blank" : undefined}
                            rel={item.external ? "noreferrer" : undefined}
                            className="rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
                        >
                            {item.label}
                        </Link>
                    ))}
                </nav>

                <div className="flex items-center gap-1.5">
                    <Button
                        asChild
                        variant="ghost"
                        size="icon"
                        className="hidden text-muted-foreground hover:text-foreground sm:inline-flex"
                    >
                        <Link
                            href={siteConfig.links.github}
                            target="_blank"
                            rel="noreferrer"
                            aria-label="GitHub"
                        >
                            <Github className="h-[18px] w-[18px]" />
                        </Link>
                    </Button>
                    <Button asChild size="sm" className="ml-1 hidden sm:inline-flex">
                        <Link href="/docs/installation">Install</Link>
                    </Button>
                    <MobileNav />
                </div>
            </div>
        </header>
    );
}
