/**
 * Best-effort parse of the autoname agent's JSON response. The agent
 * prompt asks for `{title, tags, subtitle}` JSON only, but we tolerate
 * stray prose / markdown fences by lifting the first balanced `{...}`
 * block. Mirrors the Rust-side parser in
 * `crates/attune-core/src/storage/session.rs` so every renderer
 * (Library row, Inbox preview, editor agent card) agrees on what the
 * agent produced.
 */

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
      ? obj.tags.filter((t): t is string => typeof t === "string" && t.trim().length > 0)
      : [];
    return { title, tags, subtitle };
  } catch {
    return null;
  }
}

/**
 * Returns true when the parsed autoname response is the empty
 * sentinel the prompt asks the model to emit on too-short / noisy
 * transcripts (`{"title":"","tags":[],"subtitle":""}`). Used by the
 * UI to render a "no name suggested" placeholder instead of an empty
 * card body.
 */
export function isAutonameEmpty(parsed: AutonameParsed): boolean {
  return parsed.title.length === 0 && parsed.tags.length === 0 && parsed.subtitle.length === 0;
}
