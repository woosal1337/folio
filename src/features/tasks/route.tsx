/**
 * Tasks — kanban board over the persisted task list.
 *
 * Three columns (To-do / Doing / Done), drag-and-drop between them
 * via @dnd-kit, inline "+ Add task" composer per column, click-to-edit
 * dialog with the full schema (title, owner, due, notes), trash icon
 * to delete. Tasks created by the extract-tasks agent show a sparkle
 * and a deep-link back to the source recording.
 *
 * Source of truth: useTasksStore (mirrors the Rust TaskStore). The
 * store is refreshed on first mount; mutations go through the store
 * so the agent panel's auto-extract-tasks runs show up here without
 * extra plumbing.
 */

import * as React from "react";
import {
  DndContext,
  DragOverlay,
  KeyboardSensor,
  PointerSensor,
  closestCorners,
  useDraggable,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
} from "@dnd-kit/core";
import {
  ArrowUpRight,
  Calendar,
  CheckCircle2,
  Circle,
  CircleDashed,
  Copy,
  KanbanSquare,
  Loader2,
  Plus,
  Sparkles,
  Trash2,
  User,
  X,
} from "lucide-react";
import { Link } from "react-router-dom";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { cn } from "@/shared/lib/utils";
import { copyToClipboard, taskToMarkdown } from "@/shared/lib/share";
import { useTasksStore } from "@/shared/stores/tasks-store";
import type { NewTask } from "@/shared/types/NewTask";
import type { Task } from "@/shared/types/Task";
import type { TaskStatus } from "@/shared/types/TaskStatus";
import type { TaskUpdate } from "@/shared/types/TaskUpdate";

// ts-rs generates Option<T> as `T | null` with all fields required, so
// build a New/Update with explicit nulls rather than relying on TS
// optional-property elision.
const blankNewTask = (title: string, status: TaskStatus | null): NewTask => ({
  title,
  status,
  owner: null,
  due: null,
  notes: null,
  source_session_dir: null,
  source_session_label: null,
  agent_origin: false,
});

const blankUpdate = (): TaskUpdate => ({
  title: null,
  status: null,
  owner: null,
  due: null,
  notes: null,
});

const COLUMNS: {
  id: TaskStatus;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  accent: string;
}[] = [
  { id: "todo", label: "To-do", icon: CircleDashed, accent: "text-muted-foreground" },
  { id: "doing", label: "Doing", icon: Circle, accent: "text-primary" },
  { id: "done", label: "Done", icon: CheckCircle2, accent: "text-emerald-600" },
];

export default function Tasks() {
  const tasks = useTasksStore((s) => s.tasks);
  const loading = useTasksStore((s) => s.loading);
  const refresh = useTasksStore((s) => s.refresh);
  const create = useTasksStore((s) => s.create);
  const setStatus = useTasksStore((s) => s.setStatus);
  const remove = useTasksStore((s) => s.remove);
  const update = useTasksStore((s) => s.update);

  const [editing, setEditing] = React.useState<Task | null>(null);
  const [activeDragId, setActiveDragId] = React.useState<string | null>(null);

  React.useEffect(() => {
    refresh();
  }, [refresh]);

  // Bucket tasks by status. We sort within each column by created_at
  // ascending so the user's mental order ("first task I added is at
  // the top") matches what they see.
  const byStatus = React.useMemo(() => {
    const buckets: Record<TaskStatus, Task[]> = { todo: [], doing: [], done: [] };
    for (const t of tasks) buckets[t.status].push(t);
    for (const k of Object.keys(buckets) as TaskStatus[]) {
      buckets[k].sort((a, b) => a.created_at.localeCompare(b.created_at));
    }
    return buckets;
  }, [tasks]);

  const sensors = useSensors(
    // 6px activation distance so a click doesn't accidentally start a
    // drag when the user just wants to open the edit dialog.
    useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
    useSensor(KeyboardSensor)
  );

  const activeTask = activeDragId ? tasks.find((t) => t.id === activeDragId) : null;

  const handleDragStart = (event: DragStartEvent) => {
    setActiveDragId(String(event.active.id));
  };

  const handleDragEnd = (event: DragEndEvent) => {
    setActiveDragId(null);
    const id = String(event.active.id);
    const overId = event.over?.id ? String(event.over.id) : null;
    if (!overId) return;
    // Droppables are either a column id ("todo"/"doing"/"done") or a
    // task id (when hovering another card). Normalise to a column id.
    let targetStatus: TaskStatus | null = null;
    if ((["todo", "doing", "done"] as TaskStatus[]).includes(overId as TaskStatus)) {
      targetStatus = overId as TaskStatus;
    } else {
      const overTask = tasks.find((t) => t.id === overId);
      if (overTask) targetStatus = overTask.status;
    }
    if (!targetStatus) return;
    const task = tasks.find((t) => t.id === id);
    if (!task || task.status === targetStatus) return;
    void setStatus(id, targetStatus);
  };

  return (
    <div className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-8 py-8">
      <header data-drag="" className="flex select-none items-end justify-between gap-4">
        <div>
          <h1 className="font-serif text-3xl font-medium tracking-tight">Tasks</h1>
          <p className="mt-1 text-sm text-muted-foreground">
            A board for what came out of your meetings. The Extract Tasks agent drops
            cards here automatically; you can also add your own.
          </p>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-muted-foreground">
            {tasks.length === 0
              ? "No tasks yet"
              : `${tasks.length} task${tasks.length === 1 ? "" : "s"}`}
          </span>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => refresh()}
            disabled={loading}
          >
            {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : "Refresh"}
          </Button>
        </div>
      </header>

      {loading && tasks.length === 0 ? (
        <div className="flex flex-1 items-center justify-center py-24 text-muted-foreground">
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          Loading…
        </div>
      ) : tasks.length === 0 ? (
        <EmptyState onCreate={(title) => create(blankNewTask(title, null))} />
      ) : null}

      <DndContext
        sensors={sensors}
        collisionDetection={closestCorners}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
      >
        <div className="grid flex-1 grid-cols-1 gap-4 md:grid-cols-3">
          {COLUMNS.map((col) => (
            <Column
              key={col.id}
              column={col}
              tasks={byStatus[col.id]}
              onCreate={(title) => create(blankNewTask(title, col.id))}
              onOpen={(task) => setEditing(task)}
              onDelete={(id) => remove(id)}
            />
          ))}
        </div>
        <DragOverlay>
          {activeTask ? (
            <TaskCard
              task={activeTask}
              onOpen={() => {}}
              onDelete={() => {}}
              dragging
            />
          ) : null}
        </DragOverlay>
      </DndContext>

      <EditTaskDialog
        task={editing}
        onClose={() => setEditing(null)}
        onSave={async (patch) => {
          if (!editing) return;
          await update(editing.id, patch);
          setEditing(null);
        }}
      />
      {/* end main */}
    </div>
  );
}

interface ColumnProps {
  column: (typeof COLUMNS)[number];
  tasks: Task[];
  onCreate: (title: string) => void;
  onOpen: (task: Task) => void;
  onDelete: (id: string) => void;
}

function Column({ column, tasks, onCreate, onOpen, onDelete }: ColumnProps) {
  const { setNodeRef, isOver } = useDroppable({ id: column.id });
  const Icon = column.icon;
  return (
    <section
      ref={setNodeRef}
      className={cn(
        "flex min-h-[60vh] flex-col gap-2 rounded-xl border border-border bg-secondary/40 p-3 transition-colors",
        isOver && "border-primary bg-secondary"
      )}
    >
      <header className="flex items-center justify-between px-1 py-1">
        <div className="flex items-center gap-2">
          <Icon className={cn("h-4 w-4", column.accent)} />
          <h2 className="text-sm font-medium">{column.label}</h2>
          <Badge variant="outline" className="text-2xs">
            {tasks.length}
          </Badge>
        </div>
      </header>
      <div className="flex flex-col gap-2">
        {tasks.map((task) => (
          <TaskCard
            key={task.id}
            task={task}
            onOpen={() => onOpen(task)}
            onDelete={() => onDelete(task.id)}
          />
        ))}
      </div>
      <InlineComposer onCreate={onCreate} />
    </section>
  );
}

interface TaskCardProps {
  task: Task;
  onOpen: () => void;
  onDelete: () => void;
  dragging?: boolean;
}

function TaskCard({ task, onOpen, onDelete, dragging }: TaskCardProps) {
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: task.id,
  });
  const style: React.CSSProperties = transform
    ? { transform: `translate3d(${transform.x}px, ${transform.y}px, 0)` }
    : {};
  return (
    <article
      ref={setNodeRef}
      style={style}
      {...listeners}
      {...attributes}
      className={cn(
        "group flex cursor-grab flex-col gap-1.5 rounded-lg border border-border bg-card p-3 text-left shadow-sm transition-shadow hover:shadow-md active:cursor-grabbing",
        (isDragging || dragging) && "opacity-50"
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <button
          type="button"
          onClick={(e) => {
            // Don't open while the user is dragging; @dnd-kit's pointer
            // activation distance keeps this honest.
            e.stopPropagation();
            onOpen();
          }}
          className="flex-1 text-left text-sm font-medium leading-snug"
        >
          {task.title}
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
          className="rounded p-1 text-muted-foreground opacity-0 transition-opacity hover:bg-secondary hover:text-foreground group-hover:opacity-100"
          aria-label="Delete task"
        >
          <Trash2 className="h-3.5 w-3.5" />
        </button>
      </div>
      {(task.owner || task.due) && (
        <div className="flex flex-wrap items-center gap-1.5 text-2xs text-muted-foreground">
          {task.owner && (
            <span className="inline-flex items-center gap-1 rounded bg-muted px-1.5 py-0.5">
              <User className="h-3 w-3" />
              {task.owner}
            </span>
          )}
          {task.due && (
            <span className="inline-flex items-center gap-1 rounded bg-muted px-1.5 py-0.5">
              <Calendar className="h-3 w-3" />
              {task.due}
            </span>
          )}
        </div>
      )}
      {(task.agent_origin || task.source_session_label) && (
        <footer className="mt-1 flex items-center justify-between gap-2 text-2xs text-muted-foreground">
          {task.agent_origin && (
            <span className="inline-flex items-center gap-1 text-primary">
              <Sparkles className="h-3 w-3" />
              from meeting
            </span>
          )}
          {task.source_session_label && (
            <Link
              to={`/editor/${task.source_session_label}`}
              onClick={(e) => e.stopPropagation()}
              onPointerDown={(e) => e.stopPropagation()}
              className="inline-flex items-center gap-0.5 truncate hover:text-foreground hover:underline"
              title={task.source_session_label}
            >
              <span className="max-w-[140px] truncate">
                {task.source_session_label}
              </span>
              <ArrowUpRight className="h-3 w-3 shrink-0" />
            </Link>
          )}
        </footer>
      )}
    </article>
  );
}

interface InlineComposerProps {
  onCreate: (title: string) => void;
}

function InlineComposer({ onCreate }: InlineComposerProps) {
  const [open, setOpen] = React.useState(false);
  const [value, setValue] = React.useState("");
  const ref = React.useRef<HTMLTextAreaElement | null>(null);

  React.useEffect(() => {
    if (open && ref.current) {
      ref.current.focus();
    }
  }, [open]);

  const submit = () => {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      setOpen(false);
      return;
    }
    onCreate(trimmed);
    setValue("");
    // Stay open so the user can add several in a row.
  };

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="mt-1 inline-flex w-full items-center gap-1.5 rounded-lg border border-dashed border-border bg-transparent px-3 py-2 text-xs text-muted-foreground transition-colors hover:border-primary hover:bg-card hover:text-foreground"
      >
        <Plus className="h-3.5 w-3.5" />
        Add task
      </button>
    );
  }

  return (
    <div className="flex flex-col gap-1.5 rounded-lg border border-border bg-card p-2">
      <textarea
        ref={ref}
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            submit();
          } else if (e.key === "Escape") {
            e.preventDefault();
            setValue("");
            setOpen(false);
          }
        }}
        rows={2}
        placeholder="What needs doing?"
        className="resize-none rounded border border-input bg-background px-2 py-1.5 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
      />
      <div className="flex items-center justify-between text-2xs text-muted-foreground">
        <span>↵ to add · esc to close</span>
        <button
          type="button"
          onClick={() => {
            setValue("");
            setOpen(false);
          }}
          className="rounded p-0.5 hover:bg-secondary"
          aria-label="Close composer"
        >
          <X className="h-3 w-3" />
        </button>
      </div>
    </div>
  );
}

interface EditTaskDialogProps {
  task: Task | null;
  onClose: () => void;
  onSave: (patch: TaskUpdate) => Promise<void>;
}

function EditTaskDialog({ task, onClose, onSave }: EditTaskDialogProps) {
  // Local state shadowed off the task so editing doesn't mutate the
  // store until the user hits Save.
  const [title, setTitle] = React.useState("");
  const [status, setStatus] = React.useState<TaskStatus>("todo");
  const [owner, setOwner] = React.useState("");
  const [due, setDue] = React.useState("");
  const [notes, setNotes] = React.useState("");
  const [saving, setSaving] = React.useState(false);

  React.useEffect(() => {
    if (!task) return;
    setTitle(task.title);
    setStatus(task.status);
    setOwner(task.owner ?? "");
    setDue(task.due ?? "");
    setNotes(task.notes ?? "");
    setSaving(false);
  }, [task]);

  const submit = async () => {
    if (!task) return;
    setSaving(true);
    const patch: TaskUpdate = {
      ...blankUpdate(),
      title: title.trim() || task.title,
      status,
      owner,
      due,
      notes,
    };
    await onSave(patch);
  };

  return (
    <Dialog open={!!task} onOpenChange={(open) => (!open ? onClose() : null)}>
      <DialogContent className="max-w-[520px] p-6">
        <DialogHeader>
          <DialogTitle>Edit task</DialogTitle>
        </DialogHeader>
        <div className="grid gap-4">
          <div className="grid gap-1.5">
            <Label htmlFor="task-title">Title</Label>
            <Input
              id="task-title"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
            />
          </div>
          <div className="grid grid-cols-3 gap-3">
            {COLUMNS.map((c) => (
              <button
                key={c.id}
                type="button"
                onClick={() => setStatus(c.id)}
                aria-pressed={status === c.id}
                className={cn(
                  "flex items-center justify-center gap-1.5 rounded-md border px-2 py-1.5 text-xs transition-colors",
                  status === c.id
                    ? "border-primary bg-accent"
                    : "border-border bg-card hover:bg-secondary"
                )}
              >
                <c.icon className={cn("h-3.5 w-3.5", c.accent)} />
                {c.label}
              </button>
            ))}
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="grid gap-1.5">
              <Label htmlFor="task-owner">Owner</Label>
              <Input
                id="task-owner"
                value={owner}
                onChange={(e) => setOwner(e.target.value)}
                placeholder="Ege"
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="task-due">Due</Label>
              <Input
                id="task-due"
                value={due}
                onChange={(e) => setDue(e.target.value)}
                placeholder="Friday"
              />
            </div>
          </div>
          <div className="grid gap-1.5">
            <Label htmlFor="task-notes">Notes</Label>
            <textarea
              id="task-notes"
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              rows={3}
              placeholder="Optional context"
              className="resize-none rounded-md border border-input bg-background px-3 py-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            />
          </div>
          {task?.source_session_label && (
            <div className="rounded-md bg-secondary/50 px-3 py-2 text-xs text-muted-foreground">
              From{" "}
              <Link
                to={`/editor/${task.source_session_label}`}
                className="font-medium text-foreground hover:underline"
                onClick={onClose}
              >
                {task.source_session_label}
              </Link>
            </div>
          )}
        </div>
        <DialogFooter className="sm:justify-between">
          <div className="flex items-center gap-1">
            {task ? (
              <Button
                variant="ghost"
                size="sm"
                onClick={() =>
                  task && copyToClipboard(taskToMarkdown(task), "Markdown copied")
                }
                title="Copy as Markdown checkbox"
              >
                <Copy className="mr-1.5 h-3.5 w-3.5" />
                Copy MD
              </Button>
            ) : null}
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" onClick={onClose} disabled={saving}>
              Cancel
            </Button>
            <Button onClick={submit} disabled={saving || title.trim().length === 0}>
              {saving ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
              Save
            </Button>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface EmptyStateProps {
  onCreate: (title: string) => void;
}

function EmptyState({ onCreate }: EmptyStateProps) {
  return (
    <div className="rounded-2xl border border-dashed border-border bg-card py-16">
      <div className="mx-auto flex max-w-md flex-col items-center gap-3 text-center">
        <KanbanSquare className="h-8 w-8 text-muted-foreground" />
        <h2 className="font-medium">Your board is empty</h2>
        <p className="text-sm text-muted-foreground">
          Add a task below to get started, or run the Extract Tasks agent on a recording
          to populate it from a meeting transcript.
        </p>
        <div className="mt-2 w-full max-w-xs">
          <InlineComposer onCreate={onCreate} />
        </div>
      </div>
    </div>
  );
}
