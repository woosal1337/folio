/**
 * Share helpers: render Attune artifacts as portable Markdown and
 * open them in Obsidian via the `obsidian://` URI scheme.
 *
 * The Obsidian "path" param accepts an absolute filesystem path and
 * Obsidian works out which configured vault contains it. We use
 * `path` rather than `vault=&file=` because the user may have
 * multiple vaults and we don't know which name corresponds to
 * `<memory_dir>`. v2 roadmap finding 069.
 */

import { openExternalUrl } from "@/shared/lib/ipc";
import { toast } from "sonner";

import type { Memory } from "@/shared/types/Memory";
import type { Task } from "@/shared/types/Task";

/** Build an `obsidian://open?path=<encoded>` URI for an absolute file path. */
export function obsidianHref(absolutePath: string): string {
  return `obsidian://open?path=${encodeURIComponent(absolutePath)}`;
}

/** Open a path in Obsidian. Silently no-ops if the path is empty. */
export async function openInObsidian(
  absolutePath: string | null | undefined
): Promise<void> {
  if (!absolutePath) {
    toast.error("Could not locate the source file on disk");
    return;
  }
  try {
    await openExternalUrl(obsidianHref(absolutePath));
  } catch (e) {
    console.error("openInObsidian:", e);
    toast.error("Could not open in Obsidian", { description: String(e) });
  }
}

/**
 * Render a memory as portable Markdown. Same shape as the on-disk
 * file (frontmatter + body), so pasting into Obsidian or any vault
 * round-trips cleanly with whatever frontmatter the user expects.
 */
export function memoryToMarkdown(m: Memory): string {
  const frontmatter: string[] = ["---"];
  frontmatter.push(`id: ${m.id}`);
  frontmatter.push(`kind: ${m.kind}`);
  if (m.key) frontmatter.push(`key: ${m.key}`);
  if (m.evidence) frontmatter.push(`evidence: ${quote(m.evidence)}`);
  frontmatter.push(`confidence: ${m.confidence.toFixed(2)}`);
  frontmatter.push(`tags: [${m.tags.map(quote).join(", ")}]`);
  if (m.source_session_label) {
    frontmatter.push(`source_session_label: ${m.source_session_label}`);
  }
  frontmatter.push(`valid_from: ${m.valid_from}`);
  if (m.valid_until) frontmatter.push(`valid_until: ${m.valid_until}`);
  frontmatter.push(`pinned: ${m.pinned}`);
  frontmatter.push("---");
  const heading = m.key ?? "Observation";
  return `${frontmatter.join("\n")}\n\n# ${heading}\n\n**Current:** ${m.content.trim()}\n${
    m.evidence ? `\n> ${m.evidence.trim()}\n` : ""
  }`;
}

/**
 * Render a task as portable Markdown — a single line in checkbox
 * form, with key metadata as suffix. Designed to paste into a Notes,
 * Reminders, or vault daily-note.
 */
export function taskToMarkdown(t: Task): string {
  const check = t.status === "done" ? "x" : " ";
  const parts: string[] = [`- [${check}] ${t.title}`];
  const meta: string[] = [];
  if (t.owner) meta.push(`@${t.owner}`);
  if (t.due) meta.push(`due ${t.due}`);
  if (t.source_session_label) meta.push(`from ${t.source_session_label}`);
  if (meta.length > 0) parts.push(`(${meta.join(", ")})`);
  if (t.notes) parts.push(`— ${t.notes}`);
  return parts.join(" ");
}

/** Copy text to clipboard and surface a toast. */
export async function copyToClipboard(text: string, label = "Copied"): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast.success(label);
  } catch (e) {
    console.error("copyToClipboard:", e);
    toast.error("Could not copy", { description: String(e) });
  }
}

function quote(s: string): string {
  // Same YAML-quote heuristic as the Rust side: quote if string
  // contains a YAML-special character; otherwise pass through bare.
  if (s === "" || /[:#'",\n[\]{}]/.test(s)) {
    return `"${s.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
  }
  return s;
}
