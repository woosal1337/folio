import { ActionPanel, Action, Icon, List, showToast, Toast } from "@raycast/api";
import { useEffect, useMemo, useState } from "react";
import { prefs, runCli } from "./lib/cli";

interface Memory {
  id: string;
  kind: string;
  key: string | null;
  content: string;
  tags: string[];
  source_session_label?: string | null;
  created_at: string;
}

export default function SearchMemory() {
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<Memory[]>([]);
  const [loading, setLoading] = useState(false);
  const memoryDir = useMemo(() => `${prefs().vault}/memory`, []);

  useEffect(() => {
    if (query.trim().length === 0) {
      setItems([]);
      return;
    }
    let cancelled = false;
    setLoading(true);
    runCli<Memory>(["memory-search", "--dir", memoryDir, "--limit", "30", query])
      .then((rows) => {
        if (!cancelled) setItems(rows);
      })
      .catch((e) => {
        if (!cancelled) {
          showToast({
            style: Toast.Style.Failure,
            title: "Memory search failed",
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
  }, [query, memoryDir]);

  return (
    <List
      isLoading={loading}
      onSearchTextChange={setQuery}
      searchBarPlaceholder="Search Attune memory…"
      throttle
    >
      {items.map((m) => (
        <List.Item
          key={m.id}
          icon={Icon.Brain}
          title={m.content}
          subtitle={m.key ?? undefined}
          accessories={[
            { tag: m.kind },
            ...(m.source_session_label ? [{ text: m.source_session_label }] : []),
          ]}
          actions={
            <ActionPanel>
              <Action.CopyToClipboard title="Copy Memory" content={m.content} />
              <Action.CopyToClipboard title="Copy Memory ID" content={m.id} />
            </ActionPanel>
          }
        />
      ))}
    </List>
  );
}
