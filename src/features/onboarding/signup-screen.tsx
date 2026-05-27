/**
 * Step 1 of the OTP signup flow.
 *
 * The user types their work email; on Continue we call
 * `auth_request_signin_code` which triggers the backend to email a
 * 6-digit OTP. The conductor then advances to `code-entry`.
 *
 * Per the founder's "force login" policy (2026-05-28), there is no
 * offline escape hatch — Attune requires an account to use. OAuth
 * tiles (Google / Microsoft / SSO) are shown as coming-soon
 * affordances so the layout doesn't shift when those land.
 */

import * as React from "react";
import { Loader2, ShieldCheck } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { authRequestSigninCode } from "@/shared/lib/ipc";

interface Props {
  initialEmail?: string;
  onCodeSent: (email: string) => void;
}

export function SignupScreen({ initialEmail, onCodeSent }: Props) {
  const [email, setEmail] = React.useState(initialEmail ?? "");
  const [submitting, setSubmitting] = React.useState(false);
  const [oauthPending, setOauthPending] = React.useState<string | null>(null);

  const emailLooksValid = /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email.trim());
  const canSubmit = emailLooksValid && !submitting;

  const submit = async (e?: React.FormEvent) => {
    e?.preventDefault();
    if (!canSubmit) return;
    setSubmitting(true);
    try {
      await authRequestSigninCode(email.trim().toLowerCase());
      toast.success("Code sent", {
        description: `Check ${email.trim().toLowerCase()} for a 6-digit code.`,
      });
      onCodeSent(email.trim().toLowerCase());
    } catch (e) {
      console.error("signup:", e);
      toast.error("Could not send code", { description: String(e) });
    } finally {
      setSubmitting(false);
    }
  };

  const oauthSoon = (provider: string) => {
    setOauthPending(provider);
    toast.info(`Sign in with ${provider}`, {
      description: "OAuth providers ship after the email flow stabilises. Use email for now.",
    });
    setTimeout(() => setOauthPending(null), 800);
  };

  return (
    <div className="mx-auto flex w-full max-w-md flex-col gap-7 px-6 py-12">
      <header data-drag="" className="select-none text-center">
        <div className="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <ShieldCheck className="h-5 w-5 text-primary" />
        </div>
        <h1 className="font-serif text-3xl font-medium tracking-tight">
          Welcome to Attune
        </h1>
        <p className="mt-2 text-sm text-muted-foreground">
          Sign in with your work email. We&apos;ll send you a 6-digit code —
          no password required.
        </p>
      </header>

      <form onSubmit={submit} className="flex flex-col gap-2">
        <Label htmlFor="email" className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Email address
        </Label>
        <Input
          id="email"
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder="you@company.com"
          autoComplete="email"
          autoCapitalize="none"
          autoCorrect="off"
          spellCheck={false}
          inputMode="email"
          className="h-12 text-base"
        />
        <Button
          type="submit"
          size="lg"
          disabled={!canSubmit}
          className="mt-3 h-11 gap-2"
        >
          {submitting ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Sending code…
            </>
          ) : (
            <>Continue</>
          )}
        </Button>
      </form>

      <div className="relative my-1 flex items-center">
        <div className="flex-1 border-t border-border" />
        <span className="px-3 text-2xs uppercase tracking-wider text-muted-foreground">
          Or
        </span>
        <div className="flex-1 border-t border-border" />
      </div>

      <div role="group" aria-label="Other sign-in options" className="flex flex-col gap-2">
        <ProviderButton
          provider="google"
          label="Continue with Google"
          pending={oauthPending === "google"}
          onClick={() => oauthSoon("Google")}
        >
          <GoogleGlyph />
        </ProviderButton>
        <ProviderButton
          provider="microsoft"
          label="Continue with Microsoft"
          pending={oauthPending === "microsoft"}
          onClick={() => oauthSoon("Microsoft")}
        >
          <MicrosoftGlyph />
        </ProviderButton>
        <ProviderButton
          provider="sso"
          label="Continue with SSO"
          pending={oauthPending === "sso"}
          onClick={() => oauthSoon("SSO")}
        >
          <SsoGlyph />
        </ProviderButton>
      </div>

      <p className="text-center text-2xs text-muted-foreground">
        By continuing you agree to Attune&apos;s terms of service and
        privacy policy.
      </p>
    </div>
  );
}

interface ProviderButtonProps {
  provider: string;
  label: string;
  pending: boolean;
  onClick: () => void;
  children: React.ReactNode;
}

function ProviderButton({ provider, label, pending, onClick, children }: ProviderButtonProps) {
  return (
    <Button
      variant="outline"
      size="lg"
      onClick={onClick}
      disabled={pending}
      aria-label={label}
      data-provider={provider}
      className="h-11 justify-start gap-3 px-4 text-sm font-medium"
    >
      {pending ? (
        <Loader2 className="h-4 w-4 animate-spin" />
      ) : (
        <span className="flex h-4 w-4 items-center justify-center">{children}</span>
      )}
      <span className="flex-1 text-left">{label}</span>
    </Button>
  );
}

function GoogleGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" role="img" aria-hidden="true">
      <path fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09Z" />
      <path fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.99.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84A10.99 10.99 0 0 0 12 23Z" />
      <path fill="#FBBC05" d="M5.84 14.1A6.6 6.6 0 0 1 5.49 12c0-.73.13-1.44.35-2.1V7.06H2.18a10.99 10.99 0 0 0 0 9.88l3.66-2.84Z" />
      <path fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.65l3.15-3.15C17.45 2.09 14.97 1 12 1A10.99 10.99 0 0 0 2.18 7.06l3.66 2.84C6.71 7.31 9.14 5.38 12 5.38Z" />
    </svg>
  );
}

function MicrosoftGlyph() {
  return (
    <svg viewBox="0 0 23 23" width="16" height="16" role="img" aria-hidden="true">
      <rect x="1" y="1" width="10" height="10" fill="#F25022" />
      <rect x="12" y="1" width="10" height="10" fill="#7FBA00" />
      <rect x="1" y="12" width="10" height="10" fill="#00A4EF" />
      <rect x="12" y="12" width="10" height="10" fill="#FFB900" />
    </svg>
  );
}

function SsoGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" role="img" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <rect x="3" y="11" width="18" height="11" rx="2" />
      <path d="M7 11V7a5 5 0 0 1 10 0v4" />
    </svg>
  );
}
