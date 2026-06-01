import * as React from "react";
import { toast } from "sonner";

import { getInitialDeepLink, onDeepLink } from "@/shared/lib/ipc";
import { classifyDeepLink } from "@/shared/lib/deep-link-allowlist";
import { bridgeNavigate } from "@/shared/lib/navigate-bridge";

/**
 * Handler for `attune://` deep links and external audio file drops.
 *
 * The tauri-plugin-deep-link plugin emits a single event channel that
 * carries both:
 *  - URLs the OS routed to us because they used a scheme we
 *    registered (currently just `attune://`)
 *  - file paths the OS forwarded because they matched one of the
 *    bundle's fileAssociations (currently .wav / .m4a / .mp3)
 *
 * On the first event after mount we also flush `getCurrent()` so that
 * launches caused by a deep-link or file open (the app wasn't running
 * before, the OS started it for this URL) don't lose their first
 * payload — that initial URL never fires through `onOpenUrl` if the
 * subscriber registered after the platform delivered it.
 *
 * For this initial cut we surface every received URL/path via a
 * toast so the wiring is observably correct. Full transcribe-on-drop
 * is tracked as a follow-up because importing an external audio file
 * needs a new session_dir / sidecar machinery that lives outside
 * this PR's scope. v2 finding 081 / GET-103.
 */
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

/**
 * Classify and report incoming URLs/paths. Audio file paths
 * (`file://…/foo.wav`, `/Users/.../bar.m4a`) get a "ready to
 * transcribe" toast; `attune://` URLs get a "received deep link"
 * toast. Everything else falls into a generic surface so we still
 * see noise during development.
 */
function handle(urls: string[]) {
  for (const url of urls) {
    const verdict = classifyDeepLink(url);
    switch (verdict.kind) {
      case "allowed-attune-route":
        // GET-214: route through the bridge so we never touch window.location.
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
