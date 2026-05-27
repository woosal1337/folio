/**
 * JWT signing + verification for session tokens.
 *
 * Tokens are HS256 (symmetric) — single-issuer, single-audience, no
 * key rotation infrastructure yet (revisit when we have more than
 * one Worker instance). 7-day TTL by default; the Tauri app stores
 * the token in macOS Keychain and refreshes on every backend call.
 */

import { jwtVerify, SignJWT } from "jose";
import type { Bindings } from "./env";

const ALG = "HS256";

export interface AttuneJwtClaims {
  sub: string;            // user_id
  email: string;
  email_domain: string;
  iss: string;
  aud: string;
  iat: number;
  exp: number;
  jti: string;            // for explicit revocation via KV blocklist
}

function getKey(bindings: Bindings): Uint8Array {
  const secret = bindings.JWT_SIGNING_KEY;
  if (!secret || secret.length < 32) {
    throw new Error("JWT_SIGNING_KEY missing or shorter than 32 bytes");
  }
  return new TextEncoder().encode(secret);
}

export async function signSession(
  bindings: Bindings,
  payload: { sub: string; email: string; email_domain: string },
): Promise<string> {
  const ttlDays = parseInt(bindings.SESSION_TTL_DAYS, 10) || 7;
  const now = Math.floor(Date.now() / 1000);
  const exp = now + ttlDays * 24 * 60 * 60;
  const jti = crypto.randomUUID();

  return await new SignJWT({
    email: payload.email,
    email_domain: payload.email_domain,
  })
    .setProtectedHeader({ alg: ALG })
    .setSubject(payload.sub)
    .setIssuer(bindings.JWT_ISSUER)
    .setAudience(bindings.JWT_AUDIENCE)
    .setIssuedAt(now)
    .setExpirationTime(exp)
    .setJti(jti)
    .sign(getKey(bindings));
}

export async function verifySession(
  bindings: Bindings,
  token: string,
): Promise<AttuneJwtClaims> {
  const { payload } = await jwtVerify(token, getKey(bindings), {
    issuer: bindings.JWT_ISSUER,
    audience: bindings.JWT_AUDIENCE,
    algorithms: [ALG],
  });

  // Check the KV revocation list — a logout invalidates the jti.
  if (payload.jti) {
    const revoked = await bindings.SESSIONS.get(`revoked:${payload.jti}`);
    if (revoked) {
      throw new Error("token revoked");
    }
  }

  return payload as unknown as AttuneJwtClaims;
}

export async function revokeSession(
  bindings: Bindings,
  jti: string,
  ttlSeconds: number,
): Promise<void> {
  // Store the jti in KV with a TTL matching the token's remaining
  // lifetime — after expiry the entry can be garbage-collected.
  await bindings.SESSIONS.put(`revoked:${jti}`, "1", {
    expirationTtl: Math.max(60, ttlSeconds),
  });
}
