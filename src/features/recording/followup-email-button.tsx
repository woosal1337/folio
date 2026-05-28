/**
 * GET-148 — "Write follow-up email" action.
 *
 * Runs the `write-followup-email` agent over a recording's transcript +
 * live notes and shows the draft (subject + body) in a dialog. The draft
 * is copyable and can open the default mail client via `mailto:` with the
 * subject + body prefilled.
 */

import * as React from "react";
import { Check, Copy, Loader2, Mail } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/shared/ui/dialog";
import { openExternalUrl, runAgent } from "@/shared/lib/ipc";

interface Props {
  sessionDir: string;
  /** Disabled until a transcript exists to draft from. */
  disabled: boolean;
}

/** Split the agent output into a subject line + body. */
function splitDraft(text: string): { subject: string; body: string } {
  const trimmed = text.trim();
  const m = /^subject:\s*(.+?)\r?\n([\s\S]*)$/i.exec(trimmed);
  if (m && m[1]) return { subject: m[1].trim(), body: (m[2] ?? "").trim() };
  return { subject: "Meeting follow-up", body: trimmed };
}

export function FollowupEmailButton({ sessionDir, disabled }: Props) {
  const [busy, setBusy] = React.useState(false);
  const [open, setOpen] = React.useState(false);
  const [draft, setDraft] = React.useState("");
  const [copied, setCopied] = React.useState(false);

  const run = React.useCallback(async () => {
    setBusy(true);
    try {
      const result = await runAgent(sessionDir, "write-followup-email");
      setDraft(result.response.trim());
      setOpen(true);
    } catch (e) {
      console.error("write-followup-email:", e);
      toast.error("Could not draft the email", { description: String(e) });
    } finally {
      setBusy(false);
    }
  }, [sessionDir]);

  const onCopy = React.useCallback(async () => {
    try {
      await navigator.clipboard.writeText(draft);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      toast.error("Could not copy", { description: String(e) });
    }
  }, [draft]);

  const onOpenMail = React.useCallback(() => {
    const { subject, body } = splitDraft(draft);
    const url = `mailto:?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
    openExternalUrl(url).catch((e) => {
      console.error("open mailto:", e);
      toast.error("Could not open your mail client", { description: String(e) });
    });
  }, [draft]);

  return (
    <>
      <Button
        variant="outline"
        className="gap-2"
        onClick={run}
        disabled={disabled || busy}
        aria-busy={busy}
        title={
          disabled
            ? "Generate notes first so there's a transcript to draft from"
            : "Draft a follow-up email from this meeting"
        }
      >
        {busy ? (
          <Loader2 className="h-4 w-4 animate-spin" />
        ) : (
          <Mail className="h-4 w-4" />
        )}
        Follow-up email
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>Follow-up email</DialogTitle>
            <DialogDescription>
              Drafted from the transcript and your live notes. Review before sending.
            </DialogDescription>
          </DialogHeader>
          <textarea
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            rows={14}
            aria-label="Follow-up email draft"
            className="w-full resize-y rounded-lg border border-border bg-card px-3 py-2.5 font-mono text-sm leading-relaxed focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          />
          <DialogFooter className="gap-2 sm:gap-2">
            <Button variant="outline" className="gap-2" onClick={onCopy}>
              {copied ? <Check className="h-4 w-4" /> : <Copy className="h-4 w-4" />}
              {copied ? "Copied" : "Copy"}
            </Button>
            <Button className="gap-2" onClick={onOpenMail}>
              <Mail className="h-4 w-4" />
              Open in Mail
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
