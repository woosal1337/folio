import * as React from "react";
import { useNavigate } from "react-router-dom";
import {
  Check,
  CircleUserRound,
  Cloud,
  CloudOff,
  Cpu,
  Loader2,
  Lock,
  LogOut,
  RefreshCw,
  Server,
  Sparkles,
  Zap,
} from "lucide-react";
import { toast } from "sonner";

import { Badge } from "@/shared/ui/badge";
import { Button } from "@/shared/ui/button";
import { Input } from "@/shared/ui/input";
import { Label } from "@/shared/ui/label";
import { Switch } from "@/shared/ui/switch";
import { cn } from "@/shared/lib/utils";
import { humanizeError } from "@/shared/lib/errors";
import {
  remoteLogin,
  remoteLogout,
  remoteRegister,
  testRemoteEndpoint,
  type EndpointTest,
} from "@/shared/lib/ipc";
import { useRemoteAccountStore } from "@/shared/stores/remote-account-store";
import { useSettingsStore } from "@/shared/stores/settings-store";
import { useSettingsUiStore } from "@/shared/stores/settings-ui-store";
import type { Settings } from "@/shared/types/Settings";

export default function Account() {
  const settings = useSettingsStore((s) => s.settings);
  const saveSettings = useSettingsStore((s) => s.save);
  const account = useRemoteAccountStore((s) => s.account);
  const refreshAccount = useRemoteAccountStore((s) => s.refresh);

  React.useEffect(() => {
    void refreshAccount();
  }, [refreshAccount]);

  const commit = React.useCallback(
    async <K extends keyof Settings>(key: K, value: Settings[K]) => {
      if (!settings) return;
      try {
        await saveSettings({ ...settings, [key]: value });
      } catch (e) {
        toast.error("Could not save", { description: humanizeError(e) });
      }
    },
    [settings, saveSettings]
  );

  if (!settings) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
        Loading…
      </div>
    );
  }

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-8">
      <header data-drag="" className="select-none">
        <h1 className="font-serif text-3xl font-medium tracking-tight">Account</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Your Folio Server — remote GPU transcription, sync, and sign-in.
        </p>
      </header>

      <ServerCard settings={settings} onCommit={commit} />
      <SignInCard
        endpointSet={settings.remote_endpoint.trim().length > 0}
        account={account}
        refreshAccount={refreshAccount}
      />
      <SyncCard settings={settings} onCommit={commit} signedIn={!!account?.signed_in} />
    </div>
  );
}

function SectionCard({
  icon: Icon,
  title,
  description,
  children,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  description: string;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border border-border bg-card shadow-sm">
      <div className="flex items-start gap-3 border-b border-border px-5 py-4">
        <div className="mt-0.5 rounded-md border border-border bg-muted/40 p-1.5">
          <Icon className="h-4 w-4 text-muted-foreground" />
        </div>
        <div>
          <h2 className="text-sm font-semibold">{title}</h2>
          <p className="text-xs text-muted-foreground">{description}</p>
        </div>
      </div>
      <div className="px-5 py-4">{children}</div>
    </section>
  );
}

function ServerCard({
  settings,
  onCommit,
}: {
  settings: Settings;
  onCommit: <K extends keyof Settings>(key: K, value: Settings[K]) => Promise<void>;
}) {
  const [draft, setDraft] = React.useState(settings.remote_endpoint);
  const [testing, setTesting] = React.useState(false);
  const [result, setResult] = React.useState<EndpointTest | null>(null);

  React.useEffect(() => {
    setDraft(settings.remote_endpoint);
  }, [settings.remote_endpoint]);

  const commitEndpoint = React.useCallback(async () => {
    const next = draft.trim();
    if (next === settings.remote_endpoint) return;
    await onCommit("remote_endpoint", next);
  }, [draft, settings.remote_endpoint, onCommit]);

  const runTest = React.useCallback(async (endpoint: string, quiet: boolean) => {
    const target = endpoint.trim();
    if (!target) return;
    setTesting(true);
    if (!quiet) setResult(null);
    try {
      const r = await testRemoteEndpoint(target);
      setResult(r);
      if (!quiet) {
        if (r.ok) toast.success(r.message);
        else toast.error("Could not reach server", { description: r.message });
      }
    } catch (e) {
      if (!quiet) {
        toast.error("Connection test failed", { description: humanizeError(e) });
      }
    } finally {
      setTesting(false);
    }
  }, []);

  const initialEndpoint = React.useRef(settings.remote_endpoint);
  React.useEffect(() => {
    if (initialEndpoint.current.trim()) {
      void runTest(initialEndpoint.current, true);
    }
  }, [runTest]);

  return (
    <SectionCard
      icon={Server}
      title="Server"
      description="The self-hosted Folio Server this Mac uploads to."
    >
      <div className="space-y-3">
        <div className="space-y-2">
          <Label htmlFor="account-endpoint" className="text-xs text-muted-foreground">
            Endpoint
          </Label>
          <div className="flex gap-2">
            <Input
              id="account-endpoint"
              type="url"
              placeholder="https://folio-api.example.com"
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onBlur={() => void commitEndpoint()}
              onKeyDown={(e) => {
                if (e.key === "Enter") e.currentTarget.blur();
              }}
              className="font-mono"
              autoComplete="off"
            />
            <Button
              type="button"
              variant="outline"
              onClick={async () => {
                await commitEndpoint();
                await runTest(draft, false);
              }}
              disabled={testing || draft.trim().length === 0}
            >
              {testing ? "Testing…" : "Test"}
            </Button>
          </div>
        </div>

        {result ? (
          result.ok ? (
            <div className="flex flex-wrap items-center gap-2 text-xs">
              <span className="inline-flex items-center gap-1.5 font-medium text-emerald-600 dark:text-emerald-400">
                <span className="h-1.5 w-1.5 rounded-full bg-emerald-500" />
                {result.message}
              </span>
              {result.engine ? (
                <Badge variant="outline" className="gap-1 text-2xs">
                  <Sparkles className="h-3 w-3" />
                  {result.engine}
                </Badge>
              ) : null}
              {result.model ? (
                <Badge variant="outline" className="text-2xs">
                  {result.model}
                </Badge>
              ) : null}
              <Badge
                variant="outline"
                className={cn(
                  "gap-1 text-2xs",
                  result.gpu
                    ? "border-emerald-500/40 text-emerald-600 dark:text-emerald-400"
                    : ""
                )}
              >
                <Cpu className="h-3 w-3" />
                {result.gpu ? "GPU" : "CPU"}
              </Badge>
            </div>
          ) : (
            <p className="inline-flex items-center gap-1.5 text-xs font-medium text-red-600 dark:text-red-400">
              <CloudOff className="h-3 w-3" />
              {result.message}
            </p>
          )
        ) : null}

        <p className="text-xs text-muted-foreground">
          Recordings upload to this server for GPU transcription and the transcript
          syncs back to this Mac. Deploy your own with the one-click stack in{" "}
          <code className="rounded bg-muted px-1 py-0.5 text-2xs">server/</code>.
        </p>
      </div>
    </SectionCard>
  );
}

function SignInCard({
  endpointSet,
  account,
  refreshAccount,
}: {
  endpointSet: boolean;
  account: { signed_in: boolean; email: string | null } | null;
  refreshAccount: () => Promise<void>;
}) {
  const [mode, setMode] = React.useState<"login" | "register">("login");
  const [email, setEmail] = React.useState("");
  const [password, setPassword] = React.useState("");
  const [busy, setBusy] = React.useState(false);

  const submit = async () => {
    if (email.trim().length === 0 || password.length === 0) {
      toast.error("Enter your email and password");
      return;
    }
    setBusy(true);
    try {
      if (mode === "register") {
        await remoteRegister(email.trim(), password);
        toast.success("Account created", { description: email.trim() });
      } else {
        await remoteLogin(email.trim(), password);
        toast.success("Signed in", { description: email.trim() });
      }
      setPassword("");
      await refreshAccount();
    } catch (e) {
      toast.error(mode === "register" ? "Could not register" : "Could not sign in", {
        description: humanizeError(e),
      });
    } finally {
      setBusy(false);
    }
  };

  const signOut = async () => {
    try {
      await remoteLogout();
      await refreshAccount();
      toast.success("Signed out");
    } catch (e) {
      toast.error("Could not sign out", { description: humanizeError(e) });
    }
  };

  return (
    <SectionCard
      icon={CircleUserRound}
      title="Sign-in"
      description="The server only accepts uploads from an authenticated account."
    >
      {account?.signed_in ? (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-sm font-semibold uppercase text-primary">
              {(account.email ?? "?").slice(0, 1)}
            </div>
            <div className="min-w-0">
              <p className="truncate text-sm font-medium">
                {account.email ?? "Signed in"}
              </p>
              <p className="inline-flex items-center gap-1 text-xs text-emerald-600 dark:text-emerald-400">
                <Check className="h-3 w-3" />
                Connected
              </p>
            </div>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="gap-1.5"
            onClick={() => void signOut()}
          >
            <LogOut className="h-3.5 w-3.5" />
            Sign out
          </Button>
        </div>
      ) : (
        <div className="space-y-3">
          <div
            className="inline-flex rounded-lg border border-border bg-muted/40 p-0.5"
            role="tablist"
            aria-label="Sign-in mode"
          >
            {(
              [
                { id: "login", label: "Sign in" },
                { id: "register", label: "Create account" },
              ] as const
            ).map((m) => (
              <button
                key={m.id}
                type="button"
                role="tab"
                aria-selected={mode === m.id}
                onClick={() => setMode(m.id)}
                className={cn(
                  "rounded-md px-3 py-1.5 text-xs font-medium transition-colors",
                  mode === m.id
                    ? "bg-card text-foreground shadow-sm"
                    : "text-muted-foreground hover:text-foreground"
                )}
              >
                {m.label}
              </button>
            ))}
          </div>

          <div className="grid gap-2 sm:max-w-sm">
            <Input
              type="email"
              placeholder="you@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              autoComplete="off"
              aria-label="Email"
            />
            <Input
              type="password"
              placeholder={
                mode === "register" ? "Password (min. 8 characters)" : "Password"
              }
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void submit();
              }}
              autoComplete="off"
              aria-label="Password"
            />
            <Button
              type="button"
              className="gap-2"
              onClick={() => void submit()}
              disabled={busy || !endpointSet}
            >
              {busy ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <CircleUserRound className="h-3.5 w-3.5" />
              )}
              {mode === "register" ? "Create account" : "Sign in"}
            </Button>
          </div>

          {!endpointSet ? (
            <p className="text-xs text-amber-600 dark:text-amber-400">
              Set the server endpoint above first.
            </p>
          ) : null}
          <p className="text-xs text-muted-foreground">
            Tokens are stored in the macOS Keychain and refreshed automatically —
            passwords never touch the disk.
          </p>
        </div>
      )}
    </SectionCard>
  );
}

function SyncCard({
  settings,
  onCommit,
  signedIn,
}: {
  settings: Settings;
  onCommit: <K extends keyof Settings>(key: K, value: Settings[K]) => Promise<void>;
  signedIn: boolean;
}) {
  const navigate = useNavigate();
  const openSettingsAt = useSettingsUiStore((s) => s.openAt);
  const isProvider = settings.transcriber === "remote_server";

  return (
    <SectionCard
      icon={Cloud}
      title="Sync"
      description="How recordings flow between this Mac and your server."
    >
      <div className="flex flex-col gap-4">
        <div className="flex items-center justify-between gap-4">
          <div className="space-y-0.5">
            <p className="text-sm font-medium">Transcription provider</p>
            <p className="text-xs text-muted-foreground">
              {isProvider
                ? "Remote server is the active provider — new recordings are transcribed on your GPU."
                : "Remote server is not the active provider — recordings are transcribed locally."}
            </p>
          </div>
          {isProvider ? (
            <Badge variant="accent" className="shrink-0 gap-1 text-2xs">
              <Check className="h-3 w-3" />
              Active
            </Badge>
          ) : (
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="shrink-0 gap-1.5"
              onClick={() => void onCommit("transcriber", "remote_server")}
            >
              <Zap className="h-3.5 w-3.5" />
              Make default
            </Button>
          )}
        </div>

        <div className="flex items-center justify-between gap-4">
          <div className="space-y-0.5">
            <Label htmlFor="account-auto-upload" className="text-sm font-medium">
              Auto-upload recordings
            </Label>
            <p className="text-xs text-muted-foreground">
              When a recording stops, upload it and sync the transcript back
              automatically.
            </p>
          </div>
          <Switch
            id="account-auto-upload"
            checked={settings.remote_auto_upload}
            onCheckedChange={(checked) => void onCommit("remote_auto_upload", checked)}
          />
        </div>

        {settings.privacy_mode ? (
          <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/5 px-3 py-2">
            <Lock className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-600 dark:text-amber-400" />
            <p className="text-xs text-amber-700 dark:text-amber-300">
              Privacy Mode is on — every upload is blocked until you turn it off in{" "}
              <button
                type="button"
                className="font-medium underline underline-offset-2"
                onClick={() => openSettingsAt("privacy")}
              >
                Settings → Privacy
              </button>
              .
            </p>
          </div>
        ) : null}

        {!signedIn && isProvider ? (
          <div className="flex items-start gap-2 rounded-md border border-amber-500/40 bg-amber-500/5 px-3 py-2">
            <CloudOff className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-600 dark:text-amber-400" />
            <p className="text-xs text-amber-700 dark:text-amber-300">
              Remote transcription is selected but you are not signed in — uploads will
              fail until you sign in above.
            </p>
          </div>
        ) : null}

        <div className="flex items-center gap-2 border-t border-border pt-3">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            className="gap-1.5 text-muted-foreground"
            onClick={() => navigate("/library")}
          >
            <RefreshCw className="h-3.5 w-3.5" />
            Review sync status in My Notes
          </Button>
        </div>
      </div>
    </SectionCard>
  );
}
