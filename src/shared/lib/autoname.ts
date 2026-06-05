export interface AutonameParsed {
  title: string;
  tags: string[];
  subtitle: string;
}

export function parseAutoname(response: string): AutonameParsed | null {
  const start = response.indexOf("{");
  if (start === -1) return null;
  let depth = 0;
  let inString = false;
  let escaped = false;
  let end = -1;
  for (let i = start; i < response.length; i++) {
    const ch = response[i];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === '"') {
        inString = false;
      }
      continue;
    }
    if (ch === '"') inString = true;
    else if (ch === "{") depth += 1;
    else if (ch === "}") {
      depth -= 1;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  if (end === -1) return null;
  try {
    const parsed = JSON.parse(response.slice(start, end + 1)) as unknown;
    if (!parsed || typeof parsed !== "object") return null;
    const obj = parsed as Record<string, unknown>;
    const title = typeof obj.title === "string" ? obj.title.trim() : "";
    const subtitle = typeof obj.subtitle === "string" ? obj.subtitle.trim() : "";
    const tags = Array.isArray(obj.tags)
      ? obj.tags.filter(
          (t): t is string => typeof t === "string" && t.trim().length > 0
        )
      : [];
    return { title, tags, subtitle };
  } catch {
    return null;
  }
}

export function isAutonameEmpty(parsed: AutonameParsed): boolean {
  return (
    parsed.title.length === 0 &&
    parsed.tags.length === 0 &&
    parsed.subtitle.length === 0
  );
}
