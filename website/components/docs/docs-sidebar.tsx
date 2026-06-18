"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import { cn } from "@/lib/utils";
import { docsNav } from "@/lib/docs-nav";

export function DocsSidebar({ onNavigate }: { onNavigate?: () => void }) {
  const pathname = usePathname();

  return (
    <nav className="flex flex-col gap-7" aria-label="Documentation">
      {docsNav.map((section) => (
        <div key={section.title} className="flex flex-col gap-1.5">
          <p className="px-3 font-mono text-2xs uppercase tracking-[0.16em] text-muted-foreground">
            {section.title}
          </p>
          <ul className="flex flex-col">
            {section.items.map((item) => {
              const active = pathname === item.href;
              return (
                <li key={item.href}>
                  <Link
                    href={item.href}
                    onClick={onNavigate}
                    aria-current={active ? "page" : undefined}
                    className={cn(
                      "block rounded-md px-3 py-1.5 text-ms-15 transition-colors",
                      active
                        ? "bg-accent font-medium text-accent-foreground"
                        : "text-muted-foreground hover:bg-secondary hover:text-foreground"
                    )}
                  >
                    {item.title}
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </nav>
  );
}
