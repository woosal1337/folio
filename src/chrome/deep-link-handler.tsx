import * as React from "react";
import { toast } from "sonner";

import { getInitialDeepLink, onDeepLink } from "@/shared/lib/ipc";
import { classifyDeepLink } from "@/shared/lib/deep-link-allowlist";
import { bridgeNavigate } from "@/shared/lib/navigate-bridge";

export function DeepLinkHandler() {
  React.useEffect(() => {
    let unlisten: (() => void) | null = null;

    (async () => {
      try {
        const initial = await getInitialDeepLink();
        if (initial && initial.length > 0) {
          handle(initial);
        }
      } catch (e) {
        console.warn("deep-link getCurrent failed:", e);
      }

      try {
        const off = await onDeepLink((urls) => handle(urls));
        unlisten = off;
      } catch (e) {
        console.warn("deep-link onOpenUrl subscribe failed:", e);
      }
    })();

    return () => {
      try {
        unlisten?.();
      } catch (e) {
        console.warn("deep-link unlisten failed:", e);
      }
    };
  }, []);

  return null;
}

function handle(urls: string[]) {
  for (const url of urls) {
    const verdict = classifyDeepLink(url);
    switch (verdict.kind) {
      case "allowed-attune-route":
        bridgeNavigate(verdict.route);
        toast.message("Attune deep link", {
          description: `${verdict.route}${formatParams(verdict.params)}`,
        });
        break;
      case "allowed-audio-file":
        toast.message("Audio file received", {
          description: pathLeaf(verdict.path),
          action: {
            label: "Dismiss",
            onClick: () => {},
          },
        });
        break;
      case "rejected":
        console.error("Rejected deep link:", verdict.reason, verdict.url);
        toast.error("Rejected deep link", { description: verdict.reason });
        break;
    }
  }
}

function formatParams(params: Record<string, string>): string {
  const entries = Object.entries(params);
  if (entries.length === 0) return "";
  return ` (${entries.map(([k, v]) => `${k}=${v}`).join(", ")})`;
}

function pathLeaf(path: string): string {
  return path.split("/").filter(Boolean).pop() ?? path;
}
