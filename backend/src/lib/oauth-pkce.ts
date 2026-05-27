/**
 * PKCE helpers for OAuth 2.1 flows.
 *
 * Generates a code_verifier (random 64-char URL-safe string), the
 * matching S256 code_challenge, and an opaque state token to bind
 * the callback to the originating start request.
 *
 * The verifier + state live in KV for 10 minutes; the callback
 * reads them by state and then deletes the entry (one-time-use).
 */

import type { KVNamespace } from "@cloudflare/workers-types";

const STATE_TTL_SECONDS = 600;
const VERIFIER_LENGTH = 64;

function bytesToBase64Url(bytes: Uint8Array): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/g, "");
}

function randomBase64Url(byteLength: number): string {
  const buf = new Uint8Array(byteLength);
  crypto.getRandomValues(buf);
  return bytesToBase64Url(buf);
}

async function sha256(input: string): Promise<Uint8Array> {
  const data = new TextEncoder().encode(input);
  const hash = await crypto.subtle.digest("SHA-256", data);
  return new Uint8Array(hash);
}

export interface PkceStart {
  state: string;
  code_verifier: string;
  code_challenge: string;
  code_challenge_method: "S256";
}

export interface PkceStored {
  code_verifier: string;
  provider: string;
  redirect_after_login?: string;
  referral_token?: string;
  created_at: number;
}

export async function beginPkce(
  kv: KVNamespace,
  provider: string,
  extras: { redirect_after_login?: string; referral_token?: string } = {},
): Promise<PkceStart> {
  const verifier = randomBase64Url(VERIFIER_LENGTH);
  const challengeBytes = await sha256(verifier);
  const challenge = bytesToBase64Url(challengeBytes);
  const state = randomBase64Url(32);

  const stored: PkceStored = {
    code_verifier: verifier,
    provider,
    redirect_after_login: extras.redirect_after_login,
    referral_token: extras.referral_token,
    created_at: Date.now(),
  };
  await kv.put(`pkce:${state}`, JSON.stringify(stored), {
    expirationTtl: STATE_TTL_SECONDS,
  });

  return {
    state,
    code_verifier: verifier,
    code_challenge: challenge,
    code_challenge_method: "S256",
  };
}

export async function consumePkce(
  kv: KVNamespace,
  state: string,
): Promise<PkceStored | null> {
  const raw = await kv.get(`pkce:${state}`);
  if (!raw) return null;
  // One-time use — delete immediately to defend against replay.
  await kv.delete(`pkce:${state}`);
  try {
    return JSON.parse(raw) as PkceStored;
  } catch {
    return null;
  }
}
