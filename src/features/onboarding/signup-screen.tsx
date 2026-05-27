/**
 * GET-127 — Signup screen.
 *
 * Three OAuth providers as primary buttons (Google / Microsoft / SSO)
 * plus the signature Attune fourth row "Use Attune offline" at lower
 * emphasis — Granola has no equivalent escape hatch.
 *
 * v1 ships the OAuth buttons as UI placeholders that point at the
 * attune-api backend endpoints (`/auth/google/start` etc.). The real
 * deep-link callback handler + Keychain token storage land in the
 * Sprint 2 backend pass — until then, clicking Google/Microsoft
 * surfaces a "coming soon" toast and the working path is "Use Attune
 * offline".
 *
 * Persona contracts:
 *   - Tony: three primary buttons, equal weight, the offline option
 *     is a quiet text button under a divider, not a fourth peer.
 *   - Mira: enterprise-email gate enforced server-side (when the
 *     backend lands); we don't bother validating here.
 *   - Sasha: tokens land in macOS Keychain via the existing
 *     `keyring` integration. No `localStorage`. Deep-link callback
 *     accepts the token in a URL fragment (`#token=...`), never the
 *     query string (which would log to browser history).
 *   - Kenji: all four rows are Tab-navigable in DOM order, each
 *     announces "Sign in with X, button" on focus.
 */

import * as React from "react";
import { Loader2, ShieldCheck } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";

interface Props {
  onProviderChosen: (provider: "google" | "microsoft" | "sso") => Promise<void>;
  onOffline: () => void;
}

export function SignupScreen({ onProviderChosen, onOffline }: Props) {
  const [pending, setPending] = React.useState<string | null>(null);

  const click = async (provider: "google" | "microsoft" | "sso") => {
    setPending(provider);
    try {
      await onProviderChosen(provider);
    } catch (e) {
      console.error("oauth start:", e);
      toast.error("Could not start sign-in", { description: String(e) });
    } finally {
      setPending(null);
    }
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
          Sign in with your work account to set up your workspace, or use Attune offline
          on this Mac.
        </p>
      </header>

      <div role="group" aria-label="Sign in" className="flex flex-col gap-2">
        <ProviderButton
          provider="google"
          label="Continue with Google"
          pending={pending === "google"}
          onClick={() => click("google")}
        >
          <GoogleGlyph />
        </ProviderButton>
        <ProviderButton
          provider="microsoft"
          label="Continue with Microsoft"
          pending={pending === "microsoft"}
          onClick={() => click("microsoft")}
        >
          <MicrosoftGlyph />
        </ProviderButton>
        <ProviderButton
          provider="sso"
          label="Continue with SSO"
          pending={pending === "sso"}
          onClick={() => click("sso")}
        >
          <SsoGlyph />
        </ProviderButton>
      </div>

      <div className="relative my-1 flex items-center">
        <div className="flex-1 border-t border-border" />
        <span className="px-3 text-2xs uppercase tracking-wider text-muted-foreground">
          Or
        </span>
        <div className="flex-1 border-t border-border" />
      </div>

      <button
        type="button"
        onClick={onOffline}
        className="text-center text-sm text-muted-foreground transition-colors hover:text-foreground focus-visible:underline focus-visible:outline-none"
        aria-label="Use Attune offline — no account required"
      >
        Use Attune offline
      </button>
      <p className="-mt-4 text-center text-2xs text-muted-foreground">
        No account, no cloud. Pro and Clinical features unlock when you sign in later.
      </p>

      <p className="mt-2 text-center text-2xs text-muted-foreground">
        We only accept work emails. Personal Gmail / Outlook / iCloud / Yahoo are
        blocked at signup.
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

function ProviderButton({
  provider,
  label,
  pending,
  onClick,
  children,
}: ProviderButtonProps) {
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

// Brand glyphs — kept inline as SVG so we don't introduce a brand-asset
// dependency. Each renders at 16x16. Colour follows Apple's
// sign-in-with-provider HIG: monochrome glyph on outline button.

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
