import * as React from "react";
import { ChevronRight } from "lucide-react";

import { cn } from "@/shared/lib/utils";
import {
  useContextMenu,
  type ContextMenuItem,
} from "@/shared/stores/context-menu-store";

const MENU_WIDTH = 224;
const ITEM_H = 32;

export function ContextMenu() {
  const open = useContextMenu((s) => s.open);
  const x = useContextMenu((s) => s.x);
  const y = useContextMenu((s) => s.y);
  const items = useContextMenu((s) => s.items);
  const close = useContextMenu((s) => s.close);

  React.useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close();
    };
    const onScroll = () => close();
    window.addEventListener("keydown", onKey);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", onScroll);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", onScroll);
    };
  }, [open, close]);

  if (!open) return null;

  const estHeight = items.length * ITEM_H + 8;
  const left = Math.min(x, window.innerWidth - MENU_WIDTH - 8);
  const top = Math.min(y, Math.max(8, window.innerHeight - estHeight - 8));

  return (
    <>
      <button
        type="button"
        aria-hidden="true"
        tabIndex={-1}
        className="fixed inset-0 z-[60] cursor-default"
        onClick={close}
        onContextMenu={(e) => {
          e.preventDefault();
          close();
        }}
      />
      <div
        role="menu"
        style={{ left, top }}
        className="fixed z-[61] w-56 overflow-visible rounded-md border border-border bg-popover py-1 text-sm shadow-lg"
      >
        {items.map((item) => (
          <Row key={item.id} item={item} onClose={close} />
        ))}
      </div>
    </>
  );
}

function Row({ item, onClose }: { item: ContextMenuItem; onClose: () => void }) {
  const [submenuOpen, setSubmenuOpen] = React.useState(false);
  const Icon = item.icon;
  const hasChildren = !!item.children?.length;

  const base =
    "flex w-full items-center gap-2 px-3 py-1.5 text-left transition-colors disabled:opacity-40 disabled:pointer-events-none";
  const tone = item.destructive
    ? "text-destructive hover:bg-destructive/10"
    : "text-foreground hover:bg-accent hover:text-accent-foreground";

  if (hasChildren) {
    return (
      // eslint-disable-next-line jsx-a11y/no-static-element-interactions -- hover wrapper for the submenu flyout; the row itself is a button
      <div
        className="relative"
        onMouseEnter={() => setSubmenuOpen(true)}
        onMouseLeave={() => setSubmenuOpen(false)}
      >
        <button
          type="button"
          role="menuitem"
          disabled={item.disabled}
          className={cn(base, tone, submenuOpen && "bg-accent text-accent-foreground")}
        >
          {Icon ? (
            <Icon className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
          ) : null}
          <span className="flex-1 truncate">{item.label}</span>
          <ChevronRight className="h-3.5 w-3.5 text-muted-foreground" />
        </button>
        {submenuOpen ? (
          <div
            role="menu"
            className="absolute left-full top-0 -ml-1 -mt-1 max-h-72 w-52 overflow-y-auto rounded-md border border-border bg-popover py-1 shadow-lg"
          >
            {(item.children ?? []).map((child) => (
              <Row key={child.id} item={child} onClose={onClose} />
            ))}
          </div>
        ) : null}
      </div>
    );
  }

  return (
    <>
      {item.separatorBefore ? <div className="my-1 border-t border-border" /> : null}
      <button
        type="button"
        role="menuitem"
        disabled={item.disabled}
        onClick={() => {
          onClose();
          item.onSelect?.();
        }}
        className={cn(base, tone)}
      >
        {Icon ? (
          <Icon
            className={cn(
              "h-3.5 w-3.5 shrink-0",
              item.destructive ? "" : "text-muted-foreground"
            )}
          />
        ) : null}
        <span className="flex-1 truncate">{item.label}</span>
      </button>
    </>
  );
}
