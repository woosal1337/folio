/**
 * GET-139 — Settings → Workspace → Team.
 *
 * Member list with role badges, pending invites, remove-with-confirm.
 * v1 ships UI only; the backend members + invites endpoints land
 * with attune-api so this screen renders an empty state until then.
 */

import * as React from "react";
import { Crown, Mail, RotateCcw, UserPlus, Users, X } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/shared/ui/button";
import { Label } from "@/shared/ui/label";
import type { Settings } from "@/shared/types/Settings";

interface SectionProps {
  settings: Settings;
}

export function SectionWorkspaceTeam({ settings }: SectionProps) {
  const isLocalOnly =
    settings.signin_mode === "" || settings.signin_mode === "offline";

  return (
    <section className="space-y-7">
      <header className="space-y-1">
        <h2 className="font-serif text-2xl font-medium">Team</h2>
        <p className="text-sm text-muted-foreground">
          People with access to {settings.workspace_name || "this workspace"}.
        </p>
      </header>

      {isLocalOnly ? <LocalOnlyEmpty /> : <RemoteEmpty />}

      <Group title="Members">
        <div className="rounded-lg border border-dashed border-border bg-card p-5">
          <MemberRow
            name="You"
            email={isLocalOnly ? "local account" : "signed-in email"}
            role="Owner"
            you
          />
          <p className="mt-4 text-center text-xs text-muted-foreground">
            More members appear here when teammates accept your invites.
          </p>
        </div>
      </Group>

      <Group title="Pending invites">
        <div className="flex items-start gap-3 rounded-lg border border-dashed border-border bg-card p-5">
          <Mail className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0 flex-1 space-y-0.5">
            <p className="text-sm font-medium">No pending invites</p>
            <p className="max-w-prose text-xs text-muted-foreground">
              Invites sent from the onboarding flow or with{" "}
              <span className="font-medium">Invite teammates</span> below show
              up here while waiting for the recipient to accept.
            </p>
          </div>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() =>
              toast.info("Invite teammates", {
                description:
                  "Backend invite endpoint ships with attune-api (GET-123).",
              })
            }
            className="mt-0.5 shrink-0 gap-1.5"
          >
            <UserPlus className="h-3.5 w-3.5" />
            Invite
          </Button>
        </div>
      </Group>
    </section>
  );
}

function LocalOnlyEmpty() {
  return (
    <div className="flex items-start gap-3 rounded-lg border border-amber-200 bg-amber-50 p-4 text-amber-900 dark:border-amber-900/40 dark:bg-amber-950/40 dark:text-amber-100">
      <Users className="mt-0.5 h-4 w-4 shrink-0" />
      <div className="space-y-0.5 text-sm">
        <p className="font-medium">Local-only workspace</p>
        <p className="text-xs text-amber-900/80 dark:text-amber-100/80">
          Sign in to invite teammates and share notes. Until then, this
          workspace is yours alone on this Mac.
        </p>
      </div>
    </div>
  );
}

function RemoteEmpty() {
  return (
    <div className="flex items-start gap-3 rounded-lg border border-border bg-muted/30 p-4">
      <Users className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <p className="flex-1 text-xs text-muted-foreground">
        Member management is backed by the attune-api workspace endpoints
        (GET-123). The list will populate once the backend ships.
      </p>
    </div>
  );
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="space-y-3">
      <Label className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
        {title}
      </Label>
      {children}
    </div>
  );
}

function MemberRow({
  name,
  email,
  role,
  you,
}: {
  name: string;
  email: string;
  role: "Owner" | "Admin" | "Member";
  you?: boolean;
}) {
  return (
    <div className="flex items-center gap-3 rounded-md p-2">
      <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-primary/10 text-xs font-medium text-primary">
        {name[0]?.toUpperCase() ?? "?"}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className="truncate text-sm font-medium">{name}</p>
          {you ? (
            <span className="rounded-full bg-muted px-1.5 py-0.5 text-2xs text-muted-foreground">
              you
            </span>
          ) : null}
        </div>
        <p className="truncate text-xs text-muted-foreground">{email}</p>
      </div>
      <RoleBadge role={role} />
    </div>
  );
}

function RoleBadge({ role }: { role: "Owner" | "Admin" | "Member" }) {
  const tone =
    role === "Owner"
      ? "bg-primary/10 text-primary"
      : role === "Admin"
        ? "bg-blue-100 text-blue-800 dark:bg-blue-900/40 dark:text-blue-200"
        : "bg-muted text-muted-foreground";
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-2xs font-medium uppercase tracking-wider ${tone}`}
    >
      {role === "Owner" ? <Crown className="h-3 w-3" /> : null}
      {role}
    </span>
  );
}

// Kept for the eventual "Re-send" / "Revoke" actions on pending invites.
// Linter will reactivate them when wired.
void RotateCcw;
void X;
