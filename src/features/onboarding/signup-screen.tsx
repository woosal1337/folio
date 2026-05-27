/**
 * Step 1 of the OTP signup flow.
 *
 * Granola-inspired layout: the app icon, headline, and CTA stack
 * occupy the optical centre of the window. The shell below them
 * carries the email input and the OAuth buttons in a single tight
 * column. There is no offline escape hatch — the recording surface
 * refuses to render without a valid Keychain session.
 */

import * as React from "react";
import { Loader2 } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { authRequestSigninCode } from "@/shared/lib/ipc";
import logoUrl from "@/assets/logo.svg";

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
      description: "OAuth providers ship after the email flow stabilises.",
    });
    setTimeout(() => setOauthPending(null), 800);
  };

  return (
    <div className="flex min-h-full w-full items-center justify-center px-6 py-12">
      <div className="w-full max-w-sm">
        {/* App mark — Granola-style centred icon with a soft tinted
            backdrop so it reads as a brand surface, not just an SVG. */}
        <div className="mb-8 flex justify-center" data-drag="">
          <div className="relative flex h-20 w-20 items-center justify-center rounded-3xl bg-gradient-to-br from-primary/15 via-primary/10 to-primary/5 ring-1 ring-primary/15 shadow-[0_8px_30px_-12px_hsl(var(--primary)/0.35)]">
            <img
              src={logoUrl}
              alt=""
              aria-hidden="true"
              className="h-12 w-12 object-contain"
              draggable={false}
            />
          </div>
        </div>

        <header className="mb-7 select-none text-center" data-drag="">
          <h1 className="font-serif text-[28px] font-medium leading-tight tracking-tight">
            Welcome to Attune
          </h1>
          <p className="mx-auto mt-2 max-w-[320px] text-[13px] leading-relaxed text-muted-foreground">
            Sign in with your work email. We&apos;ll send a 6-digit code — no
            password, no setup.
          </p>
        </header>

        <form onSubmit={submit} className="flex flex-col gap-2.5">
          <Label
            htmlFor="email"
            className="text-2xs font-medium uppercase tracking-[0.08em] text-muted-foreground"
          >
            Work email
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
            className="h-11 rounded-lg border-input/80 bg-card text-[14px] shadow-sm transition-colors focus-visible:border-primary/40"
          />
          <Button
            type="submit"
            size="lg"
            disabled={!canSubmit}
            className="mt-2 h-11 gap-2 rounded-lg text-[14px] font-medium shadow-sm"
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

        <div className="my-7 flex items-center">
          <div className="flex-1 border-t border-border/70" />
          <span className="px-3 text-2xs uppercase tracking-[0.12em] text-muted-foreground/70">
            or
          </span>
          <div className="flex-1 border-t border-border/70" />
        </div>

        <div
          role="group"
          aria-label="Other sign-in options"
          className="flex flex-col gap-2"
        >
          <ProviderButton
            label="Continue with Google"
            pending={oauthPending === "google"}
            onClick={() => oauthSoon("Google")}
          >
            <GoogleGlyph />
          </ProviderButton>
          <ProviderButton
            label="Continue with Microsoft"
            pending={oauthPending === "microsoft"}
            onClick={() => oauthSoon("Microsoft")}
          >
            <MicrosoftGlyph />
          </ProviderButton>
          <ProviderButton
            label="Continue with SSO"
            pending={oauthPending === "sso"}
            onClick={() => oauthSoon("SSO")}
          >
            <SsoGlyph />
          </ProviderButton>
        </div>

        <p className="mt-8 text-center text-2xs leading-relaxed text-muted-foreground/80">
          By continuing you agree to our{" "}
          <a
            href="https://attune.chele.bi/terms"
            target="_blank"
            rel="noreferrer noopener"
            className="underline-offset-2 hover:underline"
          >
            Terms
          </a>{" "}
          and{" "}
          <a
            href="https://attune.chele.bi/privacy"
            target="_blank"
            rel="noreferrer noopener"
            className="underline-offset-2 hover:underline"
          >
            Privacy Policy
          </a>
          .
        </p>
      </div>
    </div>
  );
}

interface ProviderButtonProps {
  label: string;
  pending: boolean;
  onClick: () => void;
  children: React.ReactNode;
}

function ProviderButton({ label, pending, onClick, children }: ProviderButtonProps) {
  return (
    <Button
      variant="outline"
      size="lg"
      onClick={onClick}
      disabled={pending}
      aria-label={label}
      className="h-11 justify-start gap-3 rounded-lg border-input/80 bg-card px-4 text-[13.5px] font-medium shadow-sm transition-colors hover:bg-muted/40"
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
      <path
        fill="#4285F4"
        d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09Z"
      />
      <path
        fill="#34A853"
        d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.99.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84A10.99 10.99 0 0 0 12 23Z"
      />
      <path
        fill="#FBBC05"
        d="M5.84 14.1A6.6 6.6 0 0 1 5.49 12c0-.73.13-1.44.35-2.1V7.06H2.18a10.99 10.99 0 0 0 0 9.88l3.66-2.84Z"
      />
      <path
        fill="#EA4335"
        d="M12 5.38c1.62 0 3.06.56 4.21 1.65l3.15-3.15C17.45 2.09 14.97 1 12 1A10.99 10.99 0 0 0 2.18 7.06l3.66 2.84C6.71 7.31 9.14 5.38 12 5.38Z"
      />
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
    <svg
      viewBox="0 0 24 24"
      width="16"
      height="16"
      role="img"
      aria-hidden="true"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <rect x="3" y="11" width="18" height="11" rx="2" />
      <path d="M7 11V7a5 5 0 0 1 10 0v4" />
    </svg>
  );
}
