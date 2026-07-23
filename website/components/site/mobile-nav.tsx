"use client";

import * as React from "react";
import Link from "next/link";
import { Menu, X } from "lucide-react";

import { cn } from "@/lib/utils";
import { mainNav, siteConfig } from "@/lib/site-config";
import { Button } from "@/components/ui/button";
import { Logo } from "@/components/site/logo";

export function MobileNav() {
    const [open, setOpen] = React.useState(false);

    React.useEffect(() => {
        document.body.style.overflow = open ? "hidden" : "";
        return () => {
            document.body.style.overflow = "";
        };
    }, [open]);

    return (
        <div className="md:hidden">
            <button
                type="button"
                onClick={() => setOpen(true)}
                aria-label="Open menu"
                className="focus-ring inline-flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
            >
                <Menu className="h-5 w-5" />
            </button>

            {open && (
                <div className="fixed inset-0 z-50 bg-background">
                    <div className="container flex h-16 items-center justify-between">
                        <Logo />
                        <button
                            type="button"
                            onClick={() => setOpen(false)}
                            aria-label="Close menu"
                            className="focus-ring inline-flex h-9 w-9 items-center justify-center rounded-md text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
                        >
                            <X className="h-5 w-5" />
                        </button>
                    </div>
                    <nav className="container flex flex-col gap-1 pt-6">
                        {mainNav.map((item) => (
                            <Link
                                key={item.href}
                                href={item.href}
                                onClick={() => setOpen(false)}
                                target={item.external ? "_blank" : undefined}
                                rel={item.external ? "noreferrer" : undefined}
                                className={cn(
                                    "rounded-lg px-3 py-3 text-ms-22 font-medium tracking-tight transition-colors hover:bg-accent"
                                )}
                            >
                                {item.label}
                            </Link>
                        ))}
                        <Button asChild size="lg" className="mt-6">
                            <Link
                                href="/docs/installation"
                                onClick={() => setOpen(false)}
                            >
                                Install {siteConfig.name}
                            </Link>
                        </Button>
                    </nav>
                </div>
            )}
        </div>
    );
}
