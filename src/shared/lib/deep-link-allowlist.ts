export type DeepLinkVerdict =
  | { kind: "allowed-folio-route"; route: string; params: Record<string, string> }
  | { kind: "allowed-audio-file"; path: string }
  | { kind: "rejected"; reason: string; url: string };

const FOLIO_SCHEME = "folio://";
const ALLOWED_AUDIO_EXTENSIONS = [".wav", ".m4a", ".mp3"] as const;

const ALLOWED_FOLIO_ROUTES: ReadonlySet<string> = new Set([
  "library",
  "tasks",
  "memory",
  "editor",
  "preferences",
]);

const ALLOWED_PARAMS: ReadonlySet<string> = new Set([
  "autoStart",
  "label",
  "session_dir",
  "t",
  "channel",
  "span",
]);

export function classifyDeepLink(url: string): DeepLinkVerdict {
  if (looksLikeAudio(url)) {
    return { kind: "allowed-audio-file", path: stripFileScheme(url) };
  }
  if (!url.startsWith(FOLIO_SCHEME)) {
    return { kind: "rejected", reason: "unsupported scheme", url };
  }
  const remainder = url.slice(FOLIO_SCHEME.length);
  const parts = remainder.split("?", 2);
  const pathPart = parts[0] ?? "";
  const queryPart = parts[1] ?? "";
  const segments = pathPart.split("/").filter((s) => s.length > 0);
  const head = segments[0];
  if (!head) {
    return { kind: "rejected", reason: "missing route", url };
  }
  const route = head.toLowerCase();
  if (!ALLOWED_FOLIO_ROUTES.has(route)) {
    return { kind: "rejected", reason: `unknown route '${route}'`, url };
  }
  const params: Record<string, string> = {};
  if (queryPart.length > 0) {
    for (const pair of queryPart.split("&")) {
      const [rawKey, rawValue] = pair.split("=", 2);
      if (!rawKey) continue;
      const key = decodeURIComponent(rawKey);
      if (!ALLOWED_PARAMS.has(key)) {
        return { kind: "rejected", reason: `unknown param '${key}'`, url };
      }
      params[key] = rawValue ? decodeURIComponent(rawValue) : "";
    }
  }
  if (segments.length > 1) {
    params.label = decodeURIComponent(segments.slice(1).join("/"));
  }
  return { kind: "allowed-folio-route", route, params };
}

function looksLikeAudio(url: string): boolean {
  const lower = url.toLowerCase();
  return ALLOWED_AUDIO_EXTENSIONS.some((ext) => lower.endsWith(ext));
}

function stripFileScheme(url: string): string {
  return url.replace(/^file:\/\//, "");
}
