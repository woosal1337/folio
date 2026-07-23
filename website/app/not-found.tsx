import Link from "next/link";

import { Button } from "@/components/ui/button";
import { FolioMark } from "@/components/site/logo";

export default function NotFound() {
    return (
        <div className="container flex min-h-[60vh] flex-col items-center justify-center gap-6 py-24 text-center">
            <FolioMark className="h-12 w-12" />
            <p className="font-mono text-2xs uppercase tracking-[0.18em] text-muted-foreground">
                404 — not found
            </p>
            <h1 className="font-display text-ms-34 font-semibold tracking-tight">
                This page left your machine
            </h1>
            <p className="max-w-md text-ms-15 leading-relaxed text-muted-foreground">
                The page you are looking for does not exist. Everything else is still
                local.
            </p>
            <div className="flex gap-3">
                <Button asChild>
                    <Link href="/">Back home</Link>
                </Button>
                <Button asChild variant="outline">
                    <Link href="/docs">Read the docs</Link>
                </Button>
            </div>
        </div>
    );
}
