import { ActionPanel, Action, Icon, List, open, showToast, Toast } from "@raycast/api";
import { useEffect, useMemo, useState } from "react";
import { prefs, runCli } from "./lib/cli";

interface Session {
  session_dir: string;
  label: string;
  created_at: string;
  duration_secs: number;
  has_transcript: boolean;
}

function formatDuration(secs: number): string {
  if (!Number.isFinite(secs) || secs <= 0) return "—";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

export default function RecentMeetings() {
  const [items, setItems] = useState<Session[]>([]);
  const [loading, setLoading] = useState(true);
  const recordingsDir = useMemo(() => `${prefs().vault}/recordings`, []);

  useEffect(() => {
    let cancelled = false;
    runCli<Session>(["sessions", "--output", recordingsDir, "--limit", "20"])
      .then((rows) => {
        if (!cancelled) setItems(rows);
      })
      .catch((e) => {
        if (!cancelled) {
          showToast({
            style: Toast.Style.Failure,
            title: "Could not list sessions",
            message: String(e),
          });
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [recordingsDir]);

  return (
    <List isLoading={loading} searchBarPlaceholder="Filter by label…">
      {items.map((s) => (
        <List.Item
          key={s.session_dir}
          icon={s.has_transcript ? Icon.Document : Icon.Microphone}
          title={s.label}
          subtitle={formatDuration(s.duration_secs)}
          accessories={[{ date: new Date(s.created_at) }]}
          actions={
            <ActionPanel>
              <Action
                title="Open in Folio"
                icon={Icon.AppWindow}
                onAction={() => open(`folio://editor/${encodeURIComponent(s.label)}`)}
              />
              <Action.ShowInFinder path={s.session_dir} />
              <Action.CopyToClipboard
                title="Copy Session Path"
                content={s.session_dir}
              />
            </ActionPanel>
          }
        />
      ))}
    </List>
  );
}
