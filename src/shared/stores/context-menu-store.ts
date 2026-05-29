/**
 * Lightweight right-click context menu. A caller opens it from an
 * `onContextMenu` handler with the cursor position + a list of items;
 * the host (`ContextMenu`, mounted at App root) renders the menu and
 * closes on select / click-away / Escape / scroll.
 *
 * Items may nest one level (a `children` list renders a flyout submenu,
 * e.g. "Move to folder ▸").
 */

import { create } from "zustand";
import type { ComponentType } from "react";

export interface ContextMenuItem {
  /** Stable key (and default a11y label). */
  id: string;
  label: string;
  icon?: ComponentType<{ className?: string }>;
  /** Action on select. Omit when the item only opens a submenu. */
  onSelect?: () => void;
  /** One level of submenu (e.g. folders to move into). */
  children?: ContextMenuItem[];
  destructive?: boolean;
  disabled?: boolean;
  /** Draw a divider above this item. */
  separatorBefore?: boolean;
}

interface ContextMenuState {
  open: boolean;
  x: number;
  y: number;
  items: ContextMenuItem[];
  openMenu: (x: number, y: number, items: ContextMenuItem[]) => void;
  close: () => void;
}

export const useContextMenu = create<ContextMenuState>((set) => ({
  open: false,
  x: 0,
  y: 0,
  items: [],
  openMenu: (x, y, items) => set({ open: true, x, y, items }),
  close: () => set({ open: false, items: [] }),
}));
