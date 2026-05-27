/**
 * Step 2 of the OTP signup flow.
 *
 * Shown after `auth_request_signin_code` succeeds. The user types the
 * 6-digit code emailed by the backend; on submit we call
 * `auth_verify_signin_code` which (on success) stores the tokens in
 * the Keychain and returns the user identity.
 *
 * UX details:
 *   - Six discrete numeric inputs so the user can paste a code from
 *     mail in one go (the paste handler distributes the digits) or
 *     type one digit at a time with auto-advance.
 *   - Back button returns to the signup screen so the user can fix a
 *     typo'd email without re-typing the code.
 *   - "Resend code" calls the signup endpoint again (idempotent).
 */

import * as React from "react";
import { ArrowLeft, Loader2, Mail, ShieldCheck } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import {
  authRequestSigninCode,
  authVerifySigninCode,
} from "@/shared/lib/ipc";
import { useAuthStore } from "@/shared/stores/auth-store";

interface Props {
  email: string;
  deviceId: string;
  deviceName: string;
  onBack: () => void;
  onVerified: () => void;
}

const CODE_LENGTH = 6;

export function CodeEntryScreen({
  email,
  deviceId,
  deviceName,
  onBack,
  onVerified,
}: Props) {
  const setSignedIn = useAuthStore((s) => s.setSignedIn);
  const [digits, setDigits] = React.useState<string[]>(
    Array.from({ length: CODE_LENGTH }, () => ""),
  );
  const [submitting, setSubmitting] = React.useState(false);
  const [resending, setResending] = React.useState(false);
  const inputs = React.useRef<Array<HTMLInputElement | null>>([]);

  React.useEffect(() => {
    inputs.current[0]?.focus();
  }, []);

  const code = digits.join("");
  const canSubmit = code.length === CODE_LENGTH && !submitting;

  const handleChange = (i: number, raw: string) => {
    const next = raw.replace(/\D/g, "");
    if (!next) {
      setDigits((prev) => {
        const copy = [...prev];
        copy[i] = "";
        return copy;
      });
      return;
    }
    // Pasting >1 character: distribute across the remaining boxes.
    if (next.length > 1) {
      const arr = Array.from({ length: CODE_LENGTH }, () => "");
      for (let k = 0; k < CODE_LENGTH && k < next.length; k++) {
        arr[k] = next[k]!;
      }
      setDigits(arr);
      const lastFilled = Math.min(next.length, CODE_LENGTH) - 1;
      inputs.current[lastFilled]?.focus();
      return;
    }
    setDigits((prev) => {
      const copy = [...prev];
      copy[i] = next[0] ?? "";
      return copy;
    });
    if (i < CODE_LENGTH - 1) inputs.current[i + 1]?.focus();
  };

  const handleKeyDown = (
    i: number,
    e: React.KeyboardEvent<HTMLInputElement>,
  ) => {
    if (e.key === "Backspace" && !digits[i] && i > 0) {
      inputs.current[i - 1]?.focus();
    }
    if (e.key === "Enter" && canSubmit) {
      void submit();
    }
  };

  const submit = async () => {
    if (!canSubmit) return;
    setSubmitting(true);
    try {
      const identity = await authVerifySigninCode(email, code, deviceId, deviceName);
      setSignedIn(identity);
      toast.success(`Signed in as ${identity.email}`);
      onVerified();
    } catch (e) {
      console.error("verify code:", e);
      const msg = String(e);
      toast.error("Could not verify code", { description: msg });
      // Clear the digits so the user can retype without backspacing.
      setDigits(Array.from({ length: CODE_LENGTH }, () => ""));
      inputs.current[0]?.focus();
    } finally {
      setSubmitting(false);
    }
  };

  const resend = async () => {
    setResending(true);
    try {
      await authRequestSigninCode(email);
      toast.success("New code sent", { description: `Check ${email}.` });
    } catch (e) {
      console.error("resend code:", e);
      toast.error("Could not resend", { description: String(e) });
    } finally {
      setResending(false);
    }
  };

  return (
    <div className="mx-auto flex w-full max-w-md flex-col gap-7 px-6 py-12">
      <header data-drag="" className="select-none text-center">
        <div className="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <Mail className="h-5 w-5 text-primary" />
        </div>
        <h1 className="font-serif text-3xl font-medium tracking-tight">
          Check your email
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">
          We sent a 6-digit code to <span className="font-medium">{email}</span>.
        </p>
      </header>

      <form
        className="flex flex-col items-center gap-4"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <Label className="sr-only" htmlFor="code-0">
          Sign-in code
        </Label>
        <div className="flex gap-2">
          {digits.map((d, i) => (
            <Input
              key={i}
              id={`code-${i}`}
              ref={(el) => {
                inputs.current[i] = el;
              }}
              value={d}
              onChange={(e) => handleChange(i, e.target.value)}
              onKeyDown={(e) => handleKeyDown(i, e)}
              inputMode="numeric"
              autoComplete="one-time-code"
              maxLength={CODE_LENGTH}
              className="h-12 w-10 text-center font-mono text-lg tabular-nums"
              aria-label={`Digit ${i + 1}`}
            />
          ))}
        </div>

        <Button
          type="submit"
          size="lg"
          disabled={!canSubmit}
          className="h-11 w-full gap-2"
        >
          {submitting ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Verifying…
            </>
          ) : (
            <>
              <ShieldCheck className="h-4 w-4" />
              Verify and continue
            </>
          )}
        </Button>
      </form>

      <div className="flex items-center justify-between text-xs text-muted-foreground">
        <button
          type="button"
          onClick={onBack}
          disabled={submitting}
          className="inline-flex items-center gap-1 hover:text-foreground focus-visible:outline-none focus-visible:underline"
        >
          <ArrowLeft className="h-3 w-3" />
          Change email
        </button>
        <button
          type="button"
          onClick={resend}
          disabled={resending || submitting}
          className="hover:text-foreground focus-visible:outline-none focus-visible:underline disabled:opacity-50"
        >
          {resending ? "Sending…" : "Resend code"}
        </button>
      </div>
    </div>
  );
}
