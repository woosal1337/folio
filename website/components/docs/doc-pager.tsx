"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { ArrowLeft, ArrowRight } from "lucide-react";

import { docPager } from "@/lib/docs-nav";

export function DocPager() {
    const pathname = usePathname();
    const { prev, next } = docPager(pathname);

    if (!prev && !next) {
        return null;
    }

    return (
        <nav className="mt-16 grid gap-4 border-t border-border pt-8 sm:grid-cols-2">
            {prev ? (
                <Link
                    href={prev.href}
                    className="focus-ring group flex flex-col gap-1 rounded-xl border border-border bg-card p-4 text-left shadow-sm transition-shadow hover:shadow-lift"
                >
                    <span className="flex items-center gap-1.5 font-mono text-2xs uppercase tracking-[0.14em] text-muted-foreground">
                        <ArrowLeft className="h-3 w-3" />
                        Previous
                    </span>
                    <span className="text-ms-15 font-semibold tracking-tight text-foreground">
                        {prev.title}
                    </span>
                </Link>
            ) : (
                <span />
            )}
            {next && (
                <Link
                    href={next.href}
                    className="focus-ring group flex flex-col gap-1 rounded-xl border border-border bg-card p-4 text-right shadow-sm transition-shadow hover:shadow-lift sm:items-end"
                >
                    <span className="flex items-center gap-1.5 font-mono text-2xs uppercase tracking-[0.14em] text-muted-foreground">
                        Next
                        <ArrowRight className="h-3 w-3" />
                    </span>
                    <span className="text-ms-15 font-semibold tracking-tight text-foreground">
                        {next.title}
                    </span>
                </Link>
            )}
        </nav>
    );
}
