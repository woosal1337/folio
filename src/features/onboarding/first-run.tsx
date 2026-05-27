import * as React from "react";
import {
  AudioLines,
  Brain,
  CheckCircle2,
  Cloud,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Card, CardContent } from "@/shared/ui/card";
import { cn } from "@/shared/lib/utils";
import { requestCalendarAccess, setProviderKey } from "@/shared/lib/ipc";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { useAuthStore } from "@/shared/stores/auth-store";
import { PermissionsScreen } from "./permissions-screen";
import { SignupScreen } from "./signup-screen";
import { CodeEntryScreen } from "./code-entry-screen";
import { EventKitRationaleScreen } from "./eventkit-rationale-screen";
import { WorkspaceNameScreen } from "./workspace-name-screen";
import { WorkspaceBucketScreen } from "./workspace-bucket-screen";
import { InviteTeammatesScreen } from "./invite-teammates-screen";
import { inferWorkspaceNameFromEmail } from "./infer-workspace-name";

type Transcriber = "local_whisper" | "openai";

type Bucket = "founder" | "healthcare" | "sales" | "education";

/**
 * Seven-step first-run conductor with force-login at the front.
 *
 *   permissions → signup (email) → code-entry → eventkit → workspace-name
 *                → workspace-bucket → invite-teammates → transcriber
 *
 * Per the founder's force-login policy (2026-05-28), the offline
 * escape hatch is removed — the conductor cannot exit until the user
 * has a valid Keychain session. The signup screen calls
 * `auth_request_signin_code`; the code-entry screen exchanges the OTP
 * for tokens and flips `useAuthStore.signedIn`.
 *
 * When the user signs out from Settings, the recording route detects
 * `signedIn === false` and remounts the conductor at the signup step
 * (`onboarding_completed` stays true so we don't ask for permissions
 * again — only re-authenticate).
 */

type Step =
  | "permissions"
  | "signup"
  | "code-entry"
  | "eventkit"
  | "workspace-name"
  | "workspace-bucket"
  | "invite-teammates"
  | "transcriber";

function deviceFingerprint() {
  // Stable-per-install: read or generate a random id we keep in
  // localStorage. The backend treats this as the device-id; it
  // rotates on full app reinstall. Good enough for a "manage your
  // devices" UI without OS-level fingerprinting.
  const KEY = "attune.device_id";
  const existing = window.localStorage.getItem(KEY);
  if (existing) return existing;
  const id = crypto.randomUUID().replace(/-/g, "");
  window.localStorage.setItem(KEY, id);
  return id;
}

export function FirstRunConductor({ onFinish }: { onFinish: () => void }) {
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.save);
  const onboardingCompleted = settings?.onboarding_completed ?? false;
  const signedIn = useAuthStore((s) => s.signedIn);
  const cachedIdentity = useAuthStore((s) => s.identity);

  // If onboarding finished previously and the user just signed out,
  // skip straight to signup — no need to re-grant permissions.
  const initialStep: Step = signedIn
    ? "transcriber"
    : onboardingCompleted
      ? "signup"
      : "permissions";

  const [step, setStep] = React.useState<Step>(initialStep);
  const [accountEmail, setAccountEmail] = React.useState<string | null>(
    cachedIdentity?.email ?? null,
  );
  const [workspaceName, setWorkspaceName] = React.useState<string>(
    settings?.workspace_name ?? "",
  );
  const [workspaceBucket, setWorkspaceBucket] = React.useState<Bucket | "">(
    (settings?.workspace_bucket as Bucket | "") ?? "",
  );
  const [calendarDeferred, setCalendarDeferred] = React.useState<boolean>(
    settings?.onboarding_calendar_deferred ?? false,
  );
  const [transcriber, setTranscriber] = React.useState<Transcriber>(
    (settings?.transcriber as Transcriber) ?? "local_whisper",
  );
  const [openaiKey, setOpenaiKey] = React.useState("");
  const [savingKey, setSavingKey] = React.useState(false);

  const persistPartial = React.useCallback(
    async (patch: Partial<NonNullable<typeof settings>>) => {
      if (!settings) return;
      try {
        await saveSettings({ ...settings, ...patch });
      } catch (e) {
        console.error("save settings:", e);
        toast.error("Could not save", { description: String(e) });
      }
    },
    [settings, saveSettings],
  );

  const handleCodeSent = React.useCallback((email: string) => {
    setAccountEmail(email);
    setStep("code-entry");
  }, []);

  const handleVerified = React.useCallback(async () => {
    // The auth store is already updated by CodeEntryScreen via setSignedIn.
    // Persist the email + signin_mode so Settings → Profile can show it.
    await persistPartial({
      signin_mode: "email",
    });
    setStep("eventkit");
  }, [persistPartial]);

  const handleGrantCalendar = React.useCallback(async () => {
    try {
      await requestCalendarAccess();
      setCalendarDeferred(false);
      await persistPartial({ onboarding_calendar_deferred: false });
    } catch (e) {
      console.error("request_calendar_access:", e);
      toast.error("Could not open calendar settings", { description: String(e) });
    }
    setStep("workspace-name");
  }, [persistPartial]);

  const handleSkipCalendar = React.useCallback(async () => {
    setCalendarDeferred(true);
    await persistPartial({ onboarding_calendar_deferred: true });
    setStep("workspace-name");
  }, [persistPartial]);

  const handleWorkspaceName = React.useCallback(
    async (name: string) => {
      setWorkspaceName(name);
      await persistPartial({ workspace_name: name });
      setStep("workspace-bucket");
    },
    [persistPartial],
  );

  const handleWorkspaceBucket = React.useCallback(
    async (bucket: Bucket) => {
      setWorkspaceBucket(bucket);
      await persistPartial({ workspace_bucket: bucket });
      setStep(calendarDeferred ? "transcriber" : "invite-teammates");
    },
    [persistPartial, calendarDeferred],
  );

  const handleInviteTeammates = React.useCallback(async () => {
    setStep("transcriber");
  }, []);

  const finish = React.useCallback(async () => {
    if (transcriber === "openai" && openaiKey.trim().length > 0) {
      try {
        setSavingKey(true);
        await setProviderKey("openai", openaiKey.trim());
      } catch (e) {
        console.error("set_provider_key:", e);
        toast.error("Could not save OpenAI key", { description: String(e) });
        setSavingKey(false);
        return;
      }
      setSavingKey(false);
    }
    if (!settings) return;
    try {
      await saveSettings({
        ...settings,
        workspace_name: workspaceName,
        workspace_bucket: workspaceBucket,
        onboarding_calendar_deferred: calendarDeferred,
        transcriber,
        onboarding_completed: true,
      });
      toast.success("You're set up", {
        description: "Press Cmd-R any time to start recording.",
      });
      onFinish();
    } catch (e) {
      console.error("update settings on first-run finish:", e);
      toast.error("Could not save preferences", { description: String(e) });
    }
  }, [
    openaiKey,
    settings,
    workspaceName,
    workspaceBucket,
    calendarDeferred,
    transcriber,
    saveSettings,
    onFinish,
  ]);

  if (!settings) return null;

  if (step === "permissions") {
    return <PermissionsScreen onContinue={() => setStep("signup")} />;
  }
  if (step === "signup") {
    return (
      <SignupScreen
        initialEmail={accountEmail ?? undefined}
        onCodeSent={handleCodeSent}
      />
    );
  }
  if (step === "code-entry") {
    return (
      <CodeEntryScreen
        email={accountEmail ?? ""}
        deviceId={deviceFingerprint()}
        deviceName={`Attune on ${navigator.platform || "Mac"}`}
        onBack={() => setStep("signup")}
        onVerified={() => void handleVerified()}
      />
    );
  }
  if (step === "eventkit") {
    return (
      <EventKitRationaleScreen
        onGrant={handleGrantCalendar}
        onSkip={handleSkipCalendar}
      />
    );
  }
  if (step === "workspace-name") {
    return (
      <WorkspaceNameScreen
        email={accountEmail}
        initial={workspaceName || inferWorkspaceNameFromEmail(accountEmail ?? "")}
        onContinue={handleWorkspaceName}
      />
    );
  }
  if (step === "workspace-bucket") {
    return (
      <WorkspaceBucketScreen
        initial={workspaceBucket}
        onContinue={handleWorkspaceBucket}
      />
    );
  }
  if (step === "invite-teammates") {
    const at = (accountEmail ?? "").indexOf("@");
    const domain = at >= 0 ? (accountEmail ?? "").slice(at + 1) : "";
    return (
      <InviteTeammatesScreen
        userEmail={accountEmail}
        workspaceDomain={domain}
        onContinue={() => handleInviteTeammates()}
      />
    );
  }

  // transcriber step (final)
  return (
    <div className="mx-auto flex w-full max-w-2xl flex-col gap-8 px-8 py-12">
      <header data-drag="" className="select-none">
        <div className="flex items-center gap-3">
          <Sparkles className="h-6 w-6 text-primary" />
          <h1 className="font-serif text-4xl font-medium tracking-tight">
            Welcome to Attune
          </h1>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          One last thing — pick how you want transcripts to happen.
        </p>
      </header>

      <Card>
        <CardContent className="flex flex-col gap-4 py-5">
          <div className="flex items-center gap-2">
            <Brain className="h-4 w-4 text-muted-foreground" />
            <h2 className="font-medium">Pick transcription</h2>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <TranscriberChoice
              selected={transcriber === "local_whisper"}
              onClick={() => setTranscriber("local_whisper")}
              icon={ShieldCheck}
              title="Local Whisper"
              detail="Runs on your Mac. Free. No network. Slower on first run while the model downloads."
            />
            <TranscriberChoice
              selected={transcriber === "openai"}
              onClick={() => setTranscriber("openai")}
              icon={Cloud}
              title="OpenAI Whisper"
              detail="Cloud API. Faster on long meetings. Needs your OpenAI key."
            />
          </div>
          {transcriber === "openai" ? (
            <label className="flex flex-col gap-1.5 text-sm">
              <span className="text-xs text-muted-foreground">
                OpenAI API key (stored in macOS Keychain, never on disk in plain text)
              </span>
              <input
                type="password"
                value={openaiKey}
                onChange={(e) => setOpenaiKey(e.target.value)}
                placeholder="sk-..."
                autoComplete="off"
                spellCheck={false}
                className="rounded-md border border-border bg-background px-3 py-1.5 font-mono text-xs outline-none focus:border-ring"
              />
            </label>
          ) : null}
        </CardContent>
      </Card>

      <div className={cn("flex items-center justify-between rounded-lg border border-primary/30 bg-primary/5 p-4")}>
        <div className="flex items-center gap-2">
          <CheckCircle2 className="h-4 w-4 text-primary" />
          <p className="text-sm">You can change everything later in Preferences (Cmd-,).</p>
        </div>
        <Button onClick={finish} disabled={savingKey} className="gap-2">
          <AudioLines className="h-4 w-4" />
          I&apos;m ready
        </Button>
      </div>
    </div>
  );
}

interface ChoiceProps {
  selected: boolean;
  onClick: () => void;
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  detail: string;
}

function TranscriberChoice({ selected, onClick, icon: Icon, title, detail }: ChoiceProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      aria-pressed={selected}
      className={cn(
        "flex flex-col items-start gap-2 rounded-md border p-3 text-left transition-colors",
        selected
          ? "border-primary bg-primary/5 ring-1 ring-primary/30"
          : "border-border bg-card hover:bg-muted/40",
      )}
    >
      <div className="flex items-center gap-2">
        <Icon className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium">{title}</span>
        {selected ? <CheckCircle2 className="h-3.5 w-3.5 text-primary" /> : null}
      </div>
      <p className="text-xs text-muted-foreground">{detail}</p>
    </button>
  );
}
