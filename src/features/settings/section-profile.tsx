/**
 * GET-137 — Settings → Profile + Get help.
 *
 * Account-level fields (name, avatar, email, language) and the Get
 * help link surface. Until the OAuth signup flow lands (GET-127),
 * the Profile section runs in "local-only" mode: identity is
 * synthesised from the OS user, the email field is empty, and the
 * sign-out button is hidden.
 *
 * When auth ships, this section reads from the attune-api `/users/me`
 * endpoint and writes via PATCH /users/me + clears the Keychain
 * token on Sign out.
 */

import * as React from "react";
import {
  BookOpen,
  ExternalLink,
  FileText,
  Heart,
  Keyboard,
  LogOut,
  Mail,
  User,
  Languages as LanguagesIcon,
} from "lucide-react";

import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { authLogout } from "@/shared/lib/ipc";
import { useAuthStore } from "@/shared/stores/auth-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";
import type { Settings } from "@/shared/types/Settings";

interface SectionProfileProps {
  settings: Settings;
  onChange: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
}

const LANGUAGE_OPTIONS: { value: string; label: string }[] = [
  { value: "en", label: "English" },
  { value: "de", label: "Deutsch" },
  { value: "fr", label: "Français" },
  { value: "es", label: "Español" },
  { value: "it", label: "Italiano" },
  { value: "pt", label: "Português" },
  { value: "nl", label: "Nederlands" },
  { value: "pl", label: "Polski" },
  { value: "tr", label: "Türkçe" },
  { value: "ar", label: "العربية" },
  { value: "ja", label: "日本語" },
  { value: "zh-Hans", label: "简体中文" },
];

export function SectionProfile({ settings, onChange }: SectionProfileProps) {
  // Identity is sourced from the Keychain-cached `UserIdentity` blob
  // (written on OTP verify, cleared on logout). The display-name
  // input still mirrors a local-only string until we wire
  // PATCH /api/account on save.
  const identity = useAuthStore((s) => s.identity);
  const signedIn = useAuthStore((s) => s.signedIn);
  const clearAuth = useAuthStore((s) => s.clear);
  const closeSettingsModal = useSettingsUiStore((s) => s.close);
  const [displayName, setDisplayName] = React.useState(identity?.display_name ?? "");

  React.useEffect(() => {
    setDisplayName(identity?.display_name ?? "");
  }, [identity?.display_name]);

  const handleSignOut = async () => {
    try {
      await authLogout();
    } catch (e) {
      console.error("logout:", e);
    }
    clearAuth();
    // Close the Settings modal explicitly. Without this the modal's
    // global `open` state stays true and the modal re-opens over
    // the main app the next time the user signs back in.
    closeSettingsModal();
  };

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Profile</h2>
        <p className="text-sm text-muted-foreground">
          Your identity inside Attune. Used for shared notes, agent attribution, and
          workspace membership.
        </p>
      </header>

      <Group title="Identity">
        <FieldRow icon={User} title="Display name">
          <Input
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder="Your name"
            className="max-w-xs"
          />
        </FieldRow>
        <FieldRow
          icon={Mail}
          title="Email"
          description="The address you signed in with. Becomes the workspace's primary owner."
        >
          {identity?.email ? (
            <p className="font-mono text-sm text-foreground">{identity.email}</p>
          ) : (
            <p className="text-sm italic text-muted-foreground">Not signed in</p>
          )}
        </FieldRow>
        <FieldRow
          icon={LanguagesIcon}
          title="Language"
          description="App language. Recording transcription language lives under Settings → Transcription."
        >
          <select
            value={
              settings.briefing_language === "auto" ? "en" : settings.briefing_language
            }
            onChange={(e) => onChange("briefing_language", e.target.value)}
            className="h-9 rounded-md border border-input bg-card px-2 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            {LANGUAGE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
        </FieldRow>
      </Group>

      {signedIn ? (
        <Group title="Session">
          <div className="flex items-center justify-between gap-4 rounded-md border border-border bg-card p-3">
            <div className="min-w-0 flex-1 space-y-0.5">
              <p className="text-sm font-medium">Sign out</p>
              <p className="text-xs text-muted-foreground">
                Clears tokens from this Mac&apos;s Keychain. You&apos;ll need to sign
                back in to use Attune.
              </p>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleSignOut}
              className="shrink-0 gap-1.5 border-destructive/40 text-destructive hover:bg-destructive/10 hover:text-destructive"
            >
              <LogOut className="h-3.5 w-3.5" />
              Sign out
            </Button>
          </div>
        </Group>
      ) : null}

      <Group title="Get help">
        <HelpLink
          icon={BookOpen}
          title="Documentation"
          description="Guides, tips, and how-to walkthroughs."
          href="https://attune.app/docs"
        />
        <HelpLink
          icon={Keyboard}
          title="Keyboard shortcuts"
          description="Every Cmd-, Cmd-K, and Cmd-Option-S binding in one page."
          href="https://attune.app/docs/shortcuts"
        />
        <HelpLink
          icon={Mail}
          title="Email support"
          description="Direct line to the team. We answer within a working day."
          href="mailto:support@attune.app"
        />
        <HelpLink
          icon={FileText}
          title="Open source acknowledgements"
          description="Whisper, Silero V5, Parakeet, MLX, and every other library that makes Attune possible."
          href="https://attune.app/oss"
        />
        <HelpLink
          icon={Heart}
          title="What's new"
          description="Release notes for every shipped version."
          href="https://attune.app/changelog"
        />
      </Group>
    </section>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-3">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {title}
      </Label>
      <div className="space-y-1 rounded-lg border border-border bg-card p-2">
        {children}
      </div>
    </div>
  );
}

function FieldRow({
  icon: Icon,
  title,
  description,
  children,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-start gap-4 rounded-md p-3 hover:bg-muted/30">
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1 space-y-1">
        <p className="text-sm font-medium">{title}</p>
        {description ? (
          <p className="max-w-prose text-xs text-muted-foreground">{description}</p>
        ) : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function HelpLink({
  icon: Icon,
  title,
  description,
  href,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  href: string;
}) {
  const external = href.startsWith("http");
  return (
    <a
      href={href}
      target={external ? "_blank" : undefined}
      rel={external ? "noopener noreferrer" : undefined}
      className="flex items-start gap-4 rounded-md p-3 transition-colors hover:bg-muted/30"
    >
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1 space-y-0.5">
        <p className="flex items-center gap-1.5 text-sm font-medium">
          {title}
          {external ? <ExternalLink className="h-3 w-3 text-muted-foreground" /> : null}
        </p>
        <p className="max-w-prose text-xs text-muted-foreground">{description}</p>
      </div>
    </a>
  );
}
