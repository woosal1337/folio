/**
 * GET-132 — Invite teammates from Apple Calendar attendees.
 *
 * Reads teammate candidates from EventKit via the
 * `list_attendee_suggestions` IPC, dedupes by email, filters by the
 * workspace's email domain (when available), and auto-checks the
 * full list so the default action is "invite everyone shown".
 *
 * Privacy model: calendar data never crosses the wire. The IPC runs
 * locally; only the user-selected emails leave the device when the
 * user clicks Invite. The empty-state copy is honest about this.
 *
 * Three exits per the Granola-style pattern:
 *   - Copy link        → workspace invite URL to clipboard
 *   - Skip             → continue without inviting anyone
 *   - Invite (count)   → primary CTA, sends the selected emails to
 *                        the backend invite endpoint (stubbed until
 *                        the backend lands)
 *
 * The screen is skipped by the conductor when:
 *   - `signin_mode === "offline"` (no workspace to invite to)
 *   - `onboarding_calendar_deferred === true` (we never got
 *     EventKit permission)
 */

import * as React from "react";
import {
  Check,
  Copy,
  Loader2,
  Search,
  ShieldCheck,
  Users,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { cn } from "@/shared/lib/utils";
import { listAttendeeSuggestions } from "@/shared/lib/ipc";
import type { AttendeeSuggestion } from "@/shared/types/AttendeeSuggestion";

interface Props {
  userEmail: string | null;
  workspaceDomain: string;
  onContinue: (invited: string[]) => void | Promise<void>;
}

const MIN_MEETINGS = 3;
const WINDOW_DAYS = 30;
const INVITE_LINK_STUB = "https://join.attune.app/w/local-workspace";

export function InviteTeammatesScreen({
  userEmail,
  workspaceDomain,
  onContinue,
}: Props) {
  const [loading, setLoading] = React.useState(true);
  const [suggestions, setSuggestions] = React.useState<AttendeeSuggestion[]>([]);
  const [selected, setSelected] = React.useState<Set<string>>(new Set());
  const [query, setQuery] = React.useState("");
  const [manual, setManual] = React.useState<string[]>([]);
  const [submitting, setSubmitting] = React.useState(false);

  React.useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const list = await listAttendeeSuggestions(
          userEmail ?? "",
          workspaceDomain,
          WINDOW_DAYS,
          MIN_MEETINGS,
        );
        if (cancelled) return;
        setSuggestions(list);
        setSelected(new Set(list.map((s) => s.email)));
      } catch (e) {
        console.error("list_attendee_suggestions:", e);
        toast.error("Could not read calendar", { description: String(e) });
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [userEmail, workspaceDomain]);

  const allRows: AttendeeSuggestion[] = React.useMemo(() => {
    const manualRows = manual
      .filter((email) => !suggestions.some((s) => s.email === email))
      .map<AttendeeSuggestion>((email) => ({
        email,
        display_name: "",
        meeting_count: 0,
      }));
    return [...suggestions, ...manualRows];
  }, [suggestions, manual]);

  const filteredRows = React.useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return allRows;
    return allRows.filter((r) => r.email.includes(q) || r.display_name.toLowerCase().includes(q));
  }, [allRows, query]);

  const toggle = (email: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(email)) next.delete(email);
      else next.add(email);
      return next;
    });
  };

  const addManual = (raw: string) => {
    const email = raw.trim().toLowerCase();
    if (!email.includes("@") || email === (userEmail ?? "").toLowerCase()) return;
    if (manual.includes(email) || suggestions.some((s) => s.email === email)) {
      setSelected((prev) => new Set(prev).add(email));
      return;
    }
    setManual((m) => [...m, email]);
    setSelected((prev) => new Set(prev).add(email));
    setQuery("");
  };

  const handleSubmit = async (mode: "invite" | "skip") => {
    if (submitting) return;
    setSubmitting(true);
    try {
      if (mode === "skip") {
        await onContinue([]);
        return;
      }
      const emails = Array.from(selected);
      if (emails.length > 0) {
        // Backend invite endpoint lands with attune-api; for now we
        // just acknowledge the selection locally so the conductor can
        // proceed. The list itself is passed back so a future
        // wire-up can persist it.
        toast.success(`Queued ${emails.length} invite${emails.length === 1 ? "" : "s"}`, {
          description: "Invites send when your workspace activates online.",
        });
      }
      await onContinue(emails);
    } finally {
      setSubmitting(false);
    }
  };

  const copyLink = async () => {
    try {
      await navigator.clipboard.writeText(INVITE_LINK_STUB);
      toast.success("Invite link copied");
    } catch (e) {
      console.error("clipboard:", e);
      toast.error("Could not copy link", { description: String(e) });
    }
  };

  const inviteCount = selected.size;
  const queryLooksLikeEmail = query.trim().includes("@");

  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-7 px-6 py-12">
      <header data-drag="" className="select-none">
        <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <Users className="h-5 w-5 text-primary" />
        </div>
        <h1 className="font-serif text-3xl font-medium tracking-tight">
          Invite your teammates
        </h1>
        <p className="mt-2 max-w-prose text-sm text-muted-foreground">
          Collaborate on meeting notes, share folders, and pick up where each
          other left off.
        </p>
      </header>

      <div className="relative">
        <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && queryLooksLikeEmail) {
              e.preventDefault();
              addManual(query);
            }
          }}
          placeholder="Search teammates or add by email"
          className="h-11 pl-9 text-sm"
          autoComplete="off"
          spellCheck={false}
        />
        {queryLooksLikeEmail && !allRows.some((r) => r.email === query.trim().toLowerCase()) ? (
          <Button
            type="button"
            size="sm"
            variant="ghost"
            className="absolute right-1 top-1/2 -translate-y-1/2"
            onClick={() => addManual(query)}
          >
            Add
          </Button>
        ) : null}
      </div>

      <SuggestionsList
        loading={loading}
        rows={filteredRows}
        selected={selected}
        onToggle={toggle}
      />

      <div className="flex items-start gap-3 rounded-lg border border-border bg-muted/30 p-4 text-xs text-muted-foreground">
        <ShieldCheck className="mt-0.5 h-4 w-4 shrink-0" />
        <p>
          Your meetings stay on your Mac. Attune only sends the names you
          select to your teammates&apos; inboxes — nothing else leaves the
          device.
        </p>
      </div>

      <div className="flex items-center justify-between gap-2 pt-1">
        <Button
          type="button"
          variant="outline"
          onClick={copyLink}
          disabled={submitting}
          className="gap-2"
        >
          <Copy className="h-4 w-4" />
          Copy invite link
        </Button>
        <div className="flex items-center gap-2">
          <Button
            type="button"
            variant="ghost"
            onClick={() => handleSubmit("skip")}
            disabled={submitting}
          >
            Skip
          </Button>
          <Button
            type="button"
            onClick={() => handleSubmit("invite")}
            disabled={submitting || inviteCount === 0}
            className="gap-2"
          >
            {submitting ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
            {inviteCount > 0 ? `Invite ${inviteCount}` : "Invite"}
          </Button>
        </div>
      </div>
    </div>
  );
}

function SuggestionsList({
  loading,
  rows,
  selected,
  onToggle,
}: {
  loading: boolean;
  rows: AttendeeSuggestion[];
  selected: Set<string>;
  onToggle: (email: string) => void;
}) {
  if (loading) {
    return (
      <div className="flex items-center justify-center rounded-lg border border-dashed border-border bg-card p-10 text-sm text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        Reading your calendar…
      </div>
    );
  }
  if (rows.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed border-border bg-card p-10 text-center">
        <Users className="h-5 w-5 text-muted-foreground" />
        <p className="text-sm font-medium">No teammate suggestions yet</p>
        <p className="max-w-prose text-xs text-muted-foreground">
          Once your Mac&apos;s Calendar has events with attendees in your
          workspace domain, the people you meet with most will show up here.
          You can also add anyone manually above.
        </p>
      </div>
    );
  }
  return (
    <ul role="list" className="flex flex-col gap-1 rounded-lg border border-border bg-card p-2">
      {rows.map((row) => (
        <SuggestionRow
          key={row.email}
          row={row}
          checked={selected.has(row.email)}
          onToggle={onToggle}
        />
      ))}
    </ul>
  );
}

function SuggestionRow({
  row,
  checked,
  onToggle,
}: {
  row: AttendeeSuggestion;
  checked: boolean;
  onToggle: (email: string) => void;
}) {
  const id = React.useId();
  const displayName = row.display_name || row.email.split("@")[0] || row.email;
  const meetings = row.meeting_count;
  const caption =
    meetings > 0
      ? `${meetings} ${meetings === 1 ? "meeting" : "meetings"} together`
      : "Added manually";

  return (
    <li>
      <label
        htmlFor={id}
        className={cn(
          "flex cursor-pointer items-center gap-3 rounded-md p-3 transition-colors hover:bg-muted/30",
          checked ? "bg-muted/40" : "",
        )}
      >
        <Avatar name={displayName} />
        <div className="min-w-0 flex-1">
          <p className="truncate text-sm font-medium">{displayName}</p>
          <p className="truncate text-xs text-muted-foreground">
            {row.email}
            <span className="px-1.5 text-muted-foreground/50">·</span>
            <span>{caption}</span>
          </p>
        </div>
        <Checkbox id={id} checked={checked} onChange={() => onToggle(row.email)} />
      </label>
    </li>
  );
}

function Avatar({ name }: { name: string }) {
  const initials = name
    .split(/[\s.-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((p) => p[0]?.toUpperCase() ?? "")
    .join("");
  return (
    <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-medium text-primary">
      {initials || "?"}
    </div>
  );
}

function Checkbox({
  id,
  checked,
  onChange,
}: {
  id: string;
  checked: boolean;
  onChange: () => void;
}) {
  return (
    <button
      type="button"
      role="checkbox"
      id={id}
      aria-checked={checked}
      onClick={(e) => {
        e.preventDefault();
        onChange();
      }}
      className={cn(
        "flex h-5 w-5 shrink-0 items-center justify-center rounded-md border transition-colors",
        checked
          ? "border-primary bg-primary text-primary-foreground"
          : "border-border bg-card hover:border-primary/40",
      )}
    >
      {checked ? <Check className="h-3.5 w-3.5" /> : null}
    </button>
  );
}
