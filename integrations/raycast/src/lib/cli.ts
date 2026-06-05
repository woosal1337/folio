import { getPreferenceValues } from "@raycast/api";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { homedir } from "node:os";

const execFileAsync = promisify(execFile);

interface Prefs {
  attuneCli: string;
  vault: string;
}

function expandHome(p: string): string {
  if (p.startsWith("~/")) return `${homedir()}/${p.slice(2)}`;
  if (p === "~") return homedir();
  return p;
}

export function prefs(): { cli: string; vault: string } {
  const raw = getPreferenceValues<Prefs>();
  return { cli: raw.attuneCli || "attune-cli", vault: expandHome(raw.vault) };
}

export async function runCli<T>(args: string[]): Promise<T[]> {
  const { cli } = prefs();
  const { stdout } = await execFileAsync(cli, args, { maxBuffer: 32 * 1024 * 1024 });
  const lines = stdout.split("\n").filter((l) => l.trim().length > 0);
  return lines.map((l) => JSON.parse(l) as T);
}

export async function runCliOne<T>(args: string[]): Promise<T> {
  const { cli } = prefs();
  const { stdout } = await execFileAsync(cli, args, { maxBuffer: 32 * 1024 * 1024 });
  return JSON.parse(stdout) as T;
}
