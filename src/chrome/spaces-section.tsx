import * as React from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { Folder, Plus, X } from "lucide-react";

import { cn } from "@/shared/lib/utils";
import { useFolders } from "@/shared/stores/folders-store";
import { confirmDelete } from "@/shared/stores/confirm-delete-store";

/**
 * Sidebar "Spaces" section (GET-162). Lists the user's note folders;
 * clicking one filters My Notes to it via the `?folder=` search param.
 * Supports inline create (＋), double-click rename, and hover-delete.
 * Hidden entirely in the collapsed rail to keep the icon column quiet.
 */
export function SpacesSection({ collapsed }: { collapsed: boolean }) {
  const folders = useFolders((s) => s.folders);
  const load = useFolders((s) => s.load);
  const create = useFolders((s) => s.create);
  const rename = useFolders((s) => s.rename);
  const remove = useFolders((s) => s.remove);
  const navigate = useNavigate();
  const location = useLocation();

  const activeFolder = React.useMemo(() => {
    const params = new URLSearchParams(
      location.search || location.hash.split("?")[1] || ""
    );
    return params.get("folder");
  }, [location]);

  const [adding, setAdding] = React.useState(false);
  const [draft, setDraft] = React.useState("");

  React.useEffect(() => {
    void load();
  }, [load]);

  if (collapsed) return null;

  const commitNew = async () => {
    const name = draft.trim();
    setAdding(false);
    setDraft("");
    if (!name) return;
    await create(name);
    navigate(`/library?folder=${encodeURIComponent(name)}`);
  };

  return (
    <div className="mt-4 px-2">
      <div className="flex items-center justify-between px-3 pb-1">
        <span className="text-2xs font-medium uppercase tracking-wider text-muted-foreground">
          Spaces
        </span>
        <button
          type="button"
          onClick={() => setAdding((v) => !v)}
          aria-label="New folder"
          title="New folder"
          className="rounded p-0.5 text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground"
        >
          <Plus className="h-3.5 w-3.5" />
        </button>
      </div>

      {adding && (
        <input
          // eslint-disable-next-line jsx-a11y/no-autofocus -- focus the field the user just opened to type a folder name
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
          className="mb-1 w-full rounded-md bg-transparent px-3 py-1.5 text-sm outline-none ring-1 ring-border focus:ring-ring"
        />
      )}

      <div className="space-y-0.5">
        {folders.length === 0 && !adding ? (
          <p className="px-3 py-1 text-xs text-muted-foreground/70">No folders yet.</p>
        ) : (
          folders.map((name) => (
            <FolderRow
              key={name}
              name={name}
              active={activeFolder === name}
              onOpen={() => navigate(`/library?folder=${encodeURIComponent(name)}`)}
              onRename={(next) => rename(name, next)}
              onDelete={() => remove(name)}
            />
          ))
        )}
      </div>
    </div>
  );
}

function FolderRow({
  name,
  active,
  onOpen,
  onRename,
  onDelete,
}: {
  name: string;
  active: boolean;
  onOpen: () => void;
  onRename: (next: string) => void;
  onDelete: () => void;
}) {
  const [editing, setEditing] = React.useState(false);
  const [draft, setDraft] = React.useState(name);

  React.useEffect(() => {
    if (!editing) setDraft(name);
  }, [name, editing]);

  if (editing) {
    return (
      <input
        // eslint-disable-next-line jsx-a11y/no-autofocus -- focus the inline rename field the user just activated
        autoFocus
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => {
          setEditing(false);
          const next = draft.trim();
          if (next && next !== name) onRename(next);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
          else if (e.key === "Escape") {
            setDraft(name);
            setEditing(false);
          }
        }}
        aria-label={`Rename folder ${name}`}
        className="w-full rounded-md bg-transparent px-3 py-1.5 text-sm outline-none ring-1 ring-border focus:ring-ring"
      />
    );
  }

  return (
    <div
      className={cn(
        "group flex items-center rounded-md text-sm font-medium transition-colors",
        active
          ? "bg-accent text-accent-foreground"
          : "text-muted-foreground hover:bg-accent/60 hover:text-foreground"
      )}
    >
      <button
        type="button"
        onClick={onOpen}
        onDoubleClick={() => setEditing(true)}
        className="flex min-w-0 flex-1 items-center gap-3 px-3 py-2 text-left"
        title={`${name} — double-click to rename`}
      >
        <Folder className="h-4 w-4 shrink-0" />
        <span className="truncate">{name}</span>
      </button>
      <button
        type="button"
        onClick={async () => {
          const ok = await confirmDelete({
            title: `Delete the "${name}" space?`,
            description: `The "${name}" folder is removed. Notes inside it are kept, just unfiled.`,
            confirmLabel: "Delete space",
            doubleConfirm: true,
          });
          if (ok) onDelete();
        }}
        aria-label={`Delete folder ${name}`}
        title="Delete folder"
        className="mr-1.5 rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:text-destructive group-hover:opacity-100"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
