import * as React from "react";
import { Calendar, Lock, Server, Check } from "lucide-react";

import { Button } from "@/shared/ui/button";

interface Props {
  onGrant: () => Promise<void>;
  onSkip: () => void;
}

export function EventKitRationaleScreen({ onGrant, onSkip }: Props) {
  const [granting, setGranting] = React.useState(false);

  const handleGrant = async () => {
    setGranting(true);
    try {
      await onGrant();
    } finally {
      setGranting(false);
    }
  };

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onSkip();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onSkip]);

  return (
    <div className="mx-auto flex w-full max-w-xl flex-col gap-7 px-6 py-12">
      <header data-drag="" className="select-none">
        <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <Calendar className="h-5 w-5 text-primary" />
        </div>
        <h1 className="font-serif text-3xl font-medium tracking-tight">
          Read your Mac&apos;s calendar locally
        </h1>
        <p className="mt-2 max-w-prose text-sm text-muted-foreground">
          Folio uses macOS Calendar to find the meeting you&apos;re in and name your
          recordings. We read it on your Mac. Nothing leaves the device.
        </p>
      </header>

      <ul className="space-y-3.5 rounded-lg border border-border bg-card p-5">
        <Bullet
          icon={Server}
          title="No Google or Microsoft sign-in for calendar"
          body="Whatever calendars you already added to macOS — iCloud, Google, Outlook, CalDAV — show up here automatically. We don't ask for separate OAuth."
        />
        <Bullet
          icon={Lock}
          title="Local read only"
          body="Folio calls Apple's EventKit API on this Mac. Event data is never uploaded; we don't ask for write access."
        />
        <Bullet
          icon={Check}
          title="Personal calendars stay private"
          body="Smart defaults enable work calendars only. Personal, family, and holiday calendars stay hidden until you turn them on."
        />
      </ul>

      <div className="flex flex-col gap-2">
        <Button
          size="lg"
          onClick={handleGrant}
          disabled={granting}
          className="h-11"
          aria-label="Grant calendar access"
        >
          {granting ? "Opening…" : "Grant calendar access"}
        </Button>
        <Button
          size="lg"
          variant="ghost"
          onClick={onSkip}
          disabled={granting}
          className="h-11"
          aria-label="Skip for now — set up calendar later in Settings"
        >
          Skip for now
        </Button>
      </div>

      <p className="text-center text-2xs text-muted-foreground">
        You can change this anytime in Settings → Calendar or in System Settings →
        Privacy &amp; Security → Calendar.
      </p>
    </div>
  );
}

function Bullet({
  icon: Icon,
  title,
  body,
}: {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  body: string;
}) {
  return (
    <li className="flex items-start gap-3">
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground" />
      <div className="min-w-0 flex-1">
        <p className="text-sm font-medium">{title}</p>
        <p className="mt-0.5 text-xs text-muted-foreground">{body}</p>
      </div>
    </li>
  );
}
