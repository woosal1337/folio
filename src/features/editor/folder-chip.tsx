import * as React from "react";
import { Check, Folder, FolderPlus, Plus, X } from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { useFolders } from "@/shared/stores/folders-store";

export function FolderChip({
  sessionDir,
  folder,
  onChange,
}: {
  sessionDir: string;
  folder: string | null;
  onChange: (next: string | null) => void;
}) {
  const folders = useFolders((s) => s.folders);
  const load = useFolders((s) => s.load);
  const create = useFolders((s) => s.create);
  const assign = useFolders((s) => s.assign);

  const [open, setOpen] = React.useState(false);
  const [adding, setAdding] = React.useState(false);
  const [draft, setDraft] = React.useState("");

  React.useEffect(() => {
    void load();
  }, [load]);

  const pick = async (name: string | null) => {
    setOpen(false);
    setAdding(false);
    setDraft("");
    onChange(name);
    await assign(sessionDir, name);
  };

  const commitNew = async () => {
    const name = draft.trim();
    if (!name) {
      setAdding(false);
      return;
    }
    await create(name);
    await pick(name);
  };

  return (
    <div className="relative inline-flex">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-haspopup="menu"
        aria-expanded={open}
        className={cn(
          "inline-flex items-center gap-1 rounded-full border px-2.5 py-1 transition-colors",
          folder
            ? "border-border bg-accent text-accent-foreground"
            : "border-border bg-card text-muted-foreground hover:text-foreground"
        )}
      >
        {folder ? <Folder className="h-3 w-3" /> : <FolderPlus className="h-3 w-3" />}
        {folder ?? "Add to folder"}
      </button>

      {open ? (
        <>
          <button
            type="button"
            aria-hidden="true"
            tabIndex={-1}
            className="fixed inset-0 z-10 cursor-default"
            onClick={() => setOpen(false)}
          />
          <div
            role="menu"
            className="absolute left-0 top-full z-20 mt-1 w-56 overflow-hidden rounded-md border border-border bg-popover py-1 text-sm shadow-lg"
          >
            {folders.length === 0 && !adding ? (
              <p className="px-3 py-2 text-xs text-muted-foreground">No folders yet.</p>
            ) : (
              folders.map((name) => (
                <button
                  key={name}
                  type="button"
                  role="menuitemradio"
                  aria-checked={folder === name}
                  onClick={() => void pick(name)}
                  className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
                >
                  <Folder className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                  <span className="flex-1 truncate">{name}</span>
                  {folder === name ? <Check className="h-3.5 w-3.5" /> : null}
                </button>
              ))
            )}

            <div className="my-1 border-t border-border" />

            {adding ? (
              <div className="px-2 py-1">
                <input
                  // eslint-disable-next-line jsx-a11y/no-autofocus -- focus the new-folder field the user just opened
                  autoFocus
                  value={draft}
                  onChange={(e) => setDraft(e.target.value)}
                  onBlur={commitNew}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") e.currentTarget.blur();
                    else if (e.key === "Escape") {
                      setDraft("");
                      setAdding(false);
                    }
                  }}
                  placeholder="Folder name…"
                  aria-label="New folder name"
                  className="w-full rounded-md bg-transparent px-2 py-1 text-sm outline-none ring-1 ring-border focus:ring-ring"
                />
              </div>
            ) : (
              <button
                type="button"
                role="menuitem"
                onClick={() => setAdding(true)}
                className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
              >
                <Plus className="h-3.5 w-3.5" />
                New folder…
              </button>
            )}

            {folder ? (
              <button
                type="button"
                role="menuitem"
                onClick={() => void pick(null)}
                className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-muted-foreground transition-colors hover:bg-accent hover:text-foreground"
              >
                <X className="h-3.5 w-3.5" />
                Remove from folder
              </button>
            ) : null}
          </div>
        </>
      ) : null}
    </div>
  );
}
