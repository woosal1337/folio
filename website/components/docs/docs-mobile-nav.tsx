"use client";

import * as React from "react";
import { usePathname } from "next/navigation";
import { ChevronDown, Menu } from "lucide-react";

import { docsFlat } from "@/lib/docs-nav";
import { DocsSidebar } from "@/components/docs/docs-sidebar";

export function DocsMobileNav() {
  const pathname = usePathname();
  const [open, setOpen] = React.useState(false);
  const current = docsFlat.find((item) => item.href === pathname);

  return (
    <div className="lg:hidden">
      <button
        type="button"
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        className="flex w-full items-center justify-between rounded-lg border border-border bg-card px-4 py-2.5 text-ms-15 font-medium shadow-sm focus-ring"
      >
        <span className="flex items-center gap-2">
          <Menu className="h-4 w-4 text-muted-foreground" />
          {current ? current.title : "Documentation"}
        </span>
        <ChevronDown
          className={`h-4 w-4 text-muted-foreground transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>
      {open && (
        <div className="mt-3 rounded-xl border border-border bg-card p-4 shadow-sm">
          <DocsSidebar onNavigate={() => setOpen(false)} />
        </div>
      )}
    </div>
  );
}
