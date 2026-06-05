const NETWORK_HINTS = [
  "error sending request",
  "tcp connect",
  "dns error",
  "failed to lookup",
  "no such host",
  "connection refused",
  "connection reset",
  "network is unreachable",
  "connection timed out",
  "request timed out",
  "etimedout",
];

const FRIENDLY_PATTERNS: { match: RegExp; message: string }[] = [
  {
    match: /keychain|secitem|errsec|osstatus/i,
    message: "Couldn't reach the macOS Keychain. Try again, or re-enter your key.",
  },
];

function rawMessage(e: unknown): string {
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  if (e === null || e === undefined) return "";
  if (typeof e === "object") {
    const o = e as Record<string, unknown>;
    for (const key of ["message", "error", "reason", "detail"]) {
      if (typeof o[key] === "string") return o[key] as string;
    }
    try {
      const json = JSON.stringify(e);
      return json === "{}" || json === "[]" ? "" : json;
    } catch {
      return "";
    }
  }
  return String(e);
}

export function humanizeError(e: unknown): string {
  let msg = rawMessage(e).trim();
  msg = msg.replace(/^ipc\s+[\w-]+\s+failed:\s*/i, "");
  msg = msg.replace(/^(IpcError|Error|RuntimeError|FolioError):\s*/i, "");
  msg = msg.trim();

  const lower = msg.toLowerCase();
  if (NETWORK_HINTS.some((h) => lower.includes(h))) {
    return "Network problem. Check your connection and try again.";
  }
  for (const { match, message } of FRIENDLY_PATTERNS) {
    if (match.test(msg)) return message;
  }

  if (msg.length === 0) return "Something went wrong. Please try again.";
  return msg.charAt(0).toUpperCase() + msg.slice(1);
}
