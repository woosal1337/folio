import { getPreferenceValues } from "@raycast/api";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { homedir } from "node:os";

const execFileAsync = promisify(execFile);

interface Prefs {
  attuneCli: string;
  vault: string;
}

/** Expand `~` to the user's home directory. The Raycast preferences UI
 *  hands back the literal string with the tilde, not the resolved path. */
function expandHome(p: string): string {
  if (p.startsWith("~/")) return `${homedir()}/${p.slice(2)}`;
  if (p === "~") return homedir();
  return p;
}

export function prefs(): { cli: string; vault: string } {
  const raw = getPreferenceValues<Prefs>();
  return { cli: raw.attuneCli || "attune-cli", vault: expandHome(raw.vault) };
}

/** Spawn `attune-cli <args>` and return parsed JSON. NDJSON output (one
 *  object per line) is reassembled into an array. */
export async function runCli<T>(args: string[]): Promise<T[]> {
  const { cli } = prefs();
  const { stdout } = await execFileAsync(cli, args, { maxBuffer: 32 * 1024 * 1024 });
  const lines = stdout.split("\n").filter((l) => l.trim().length > 0);
  return lines.map((l) => JSON.parse(l) as T);
}

/** Same as runCli but for a single JSON document on stdout. */
export async function runCliOne<T>(args: string[]): Promise<T> {
  const { cli } = prefs();
  const { stdout } = await execFileAsync(cli, args, { maxBuffer: 32 * 1024 * 1024 });
  return JSON.parse(stdout) as T;
}
