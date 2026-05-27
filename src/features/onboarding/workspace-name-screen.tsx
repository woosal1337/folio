/**
 * GET-131 — Workspace name.
 *
 * Auto-populated from the signed-in email domain via
 * inferWorkspaceNameFromEmail. The user can rename; we just need
 * a non-empty trimmed value before Continue.
 *
 * Skipped entirely when signin_mode === "offline" (the conductor
 * routes around this screen).
 *
 * Tony: input is large (h-12, 18px text). Continue is the primary
 * CTA, disabled until the field passes validation. Enter submits.
 * Mira: we do not validate domain claims here — that happens server-
 * side when the workspace is actually created.
 */

import * as React from "react";
import { Building2 } from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { inferWorkspaceNameFromEmail } from "./infer-workspace-name";

interface Props {
  email: string | null;
  initial?: string;
  onContinue: (name: string) => void | Promise<void>;
}

export function WorkspaceNameScreen({ email, initial, onContinue }: Props) {
  const inferred = React.useMemo(
    () => initial ?? inferWorkspaceNameFromEmail(email ?? ""),
    [email, initial]
  );
  const [name, setName] = React.useState(inferred);
  const [submitting, setSubmitting] = React.useState(false);
  const inputRef = React.useRef<HTMLInputElement | null>(null);

  React.useEffect(() => {
    // Pre-select the inferred value so a quick keystroke replaces it.
    const t = setTimeout(() => {
      inputRef.current?.focus();
      inputRef.current?.select();
    }, 50);
    return () => clearTimeout(t);
  }, []);

  const trimmed = name.trim();
  const canContinue = trimmed.length > 0 && trimmed.length <= 64 && !submitting;

  const submit = async () => {
    if (!canContinue) return;
    setSubmitting(true);
    try {
      await onContinue(trimmed);
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form
      className="mx-auto flex w-full max-w-md flex-col gap-7 px-6 py-12"
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
    >
      <header data-drag="" className="select-none">
        <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <Building2 className="h-5 w-5 text-primary" />
        </div>
        <h1 className="font-serif text-3xl font-medium tracking-tight">
          Name your workspace
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Teammates with a matching work email can join later. You can rename this
          anytime.
        </p>
      </header>

      <div className="flex flex-col gap-2">
        <Label
          htmlFor="workspace-name"
          className="text-xs font-medium uppercase tracking-wider text-muted-foreground"
        >
          Workspace name
        </Label>
        <Input
          id="workspace-name"
          ref={inputRef}
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="e.g. Clinora"
          maxLength={64}
          autoComplete="off"
          spellCheck={false}
          aria-invalid={trimmed.length === 0}
          className="h-12 text-base"
        />
        <p className="text-2xs text-muted-foreground">{trimmed.length}/64</p>
      </div>

      <Button type="submit" size="lg" disabled={!canContinue} className="h-11">
        {submitting ? "Saving…" : "Continue"}
      </Button>
    </form>
  );
}
