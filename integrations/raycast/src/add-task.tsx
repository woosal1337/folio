import { Action, ActionPanel, Form, popToRoot, showToast, Toast } from "@raycast/api";
import { useState } from "react";
import { writeFile, readFile, mkdir } from "node:fs/promises";
import { existsSync } from "node:fs";
import { dirname } from "node:path";
import { randomUUID } from "node:crypto";
import { prefs } from "./lib/cli";

interface Task {
  id: string;
  title: string;
  status: "todo" | "doing" | "done";
  owner: string | null;
  due: string | null;
  notes: string | null;
  source_session_dir: string | null;
  source_session_label: string | null;
  agent_origin: boolean;
  created_at: string;
  updated_at: string;
}

export default function AddTask() {
  const [submitting, setSubmitting] = useState(false);

  return (
    <Form
      isLoading={submitting}
      actions={
        <ActionPanel>
          <Action.SubmitForm
            title="Add Task"
            onSubmit={async (values: {
              title: string;
              owner?: string;
              due?: string;
              notes?: string;
            }) => {
              if (!values.title?.trim()) {
                await showToast({
                  style: Toast.Style.Failure,
                  title: "Title is required",
                });
                return;
              }
              setSubmitting(true);
              try {
                const tasksPath = `${prefs().vault}/tasks/tasks.json`;
                await mkdir(dirname(tasksPath), { recursive: true });
                const existing: Task[] = existsSync(tasksPath)
                  ? JSON.parse(await readFile(tasksPath, "utf8"))
                  : [];
                const now = new Date().toISOString();
                const task: Task = {
                  id: randomUUID(),
                  title: values.title.trim(),
                  status: "todo",
                  owner: values.owner?.trim() || null,
                  due: values.due?.trim() || null,
                  notes: values.notes?.trim() || null,
                  source_session_dir: null,
                  source_session_label: null,
                  agent_origin: false,
                  created_at: now,
                  updated_at: now,
                };
                existing.push(task);
                await writeFile(tasksPath, JSON.stringify(existing, null, 2));
                await showToast({
                  style: Toast.Style.Success,
                  title: "Task added",
                  message: task.title,
                });
                await popToRoot();
              } catch (e) {
                await showToast({
                  style: Toast.Style.Failure,
                  title: "Could not add task",
                  message: String(e),
                });
              } finally {
                setSubmitting(false);
              }
            }}
          />
        </ActionPanel>
      }
    >
      <Form.TextField
        id="title"
        title="Title"
        placeholder="Reply to design review"
        autoFocus
      />
      <Form.TextField id="owner" title="Owner" placeholder="ege" />
      <Form.TextField
        id="due"
        title="Due"
        placeholder="Friday, 2026-06-01, next sprint…"
      />
      <Form.TextArea id="notes" title="Notes" placeholder="Optional context" />
    </Form>
  );
}
