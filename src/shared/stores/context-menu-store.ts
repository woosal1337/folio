import { create } from "zustand";
import type { ComponentType } from "react";

export interface ContextMenuItem {
  id: string;
  label: string;
  icon?: ComponentType<{ className?: string }>;

  onSelect?: () => void;

  children?: ContextMenuItem[];
  destructive?: boolean;
  disabled?: boolean;

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
