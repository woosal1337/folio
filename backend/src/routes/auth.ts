/**
 * OAuth + session routes.
 *
 *   POST /auth/google/start         → returns the Google auth URL
 *   GET  /auth/google/callback      → handles the OAuth code exchange
 *   POST /auth/microsoft/start      → returns the Microsoft auth URL
 *   GET  /auth/microsoft/callback   → handles the Microsoft code exchange
 *   POST /auth/logout               → revokes the bearer token
 *
 * Both providers use the OAuth 2.1 / PKCE flow. State + verifier live
 * in KV for 10 min; the callback consumes them one-time and exchanges
 * the code for an ID token. We DO NOT request scopes beyond `openid
 * email profile` here — calendar scopes are requested later via the
 * pre-rationale flow (GET-128), not at signup.
 */

import { Hono } from "hono";
import { HTTPException } from "hono/http-exception";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import type { Env } from "../lib/env";
import { beginPkce, consumePkce } from "../lib/oauth-pkce";
import { signSession, verifySession, revokeSession } from "../lib/jwt";
import { requireAuth } from "../lib/auth-middleware";

export const authRoutes = new Hono<Env>();

// Blocklist of personal-email providers per Mira's enterprise-gate rule.
const PERSONAL_EMAIL_DOMAINS = new Set([
  "gmail.com", "googlemail.com", "outlook.com", "hotmail.com",
  "live.com", "yahoo.com", "icloud.com", "me.com", "mac.com",
  "aol.com", "proton.me", "protonmail.com",
]);

const startSchema = z.object({
  redirect_after_login: z.string().url().optional(),
  referral_token: z.string().optional(),
});

// ------------------------------------------------------------------
// Google
// ------------------------------------------------------------------
const GOOGLE_AUTH_URL = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL = "https://oauth2.googleapis.com/token";
const GOOGLE_USERINFO_URL = "https://openidconnect.googleapis.com/v1/userinfo";

authRoutes.post(
  "/google/start",
  zValidator("json", startSchema),
  async (c) => {
    const body = c.req.valid("json");
    const pkce = await beginPkce(c.env.SESSIONS, "google", body);

    const url = new URL(GOOGLE_AUTH_URL);
    url.searchParams.set("client_id", c.env.GOOGLE_OAUTH_CLIENT_ID);
    url.searchParams.set("response_type", "code");
    url.searchParams.set("scope", "openid email profile");
    url.searchParams.set("redirect_uri", `${c.env.JWT_ISSUER}/auth/google/callback`);
    url.searchParams.set("state", pkce.state);
    url.searchParams.set("code_challenge", pkce.code_challenge);
    url.searchParams.set("code_challenge_method", pkce.code_challenge_method);
    // Defend against silent-login in case the user has multiple Google accounts.
    url.searchParams.set("prompt", "select_account");

    return c.json({ auth_url: url.toString(), state: pkce.state });
  },
);

authRoutes.get("/google/callback", async (c) => {
  const code = c.req.query("code");
  const state = c.req.query("state");
  if (!code || !state) {
    throw new HTTPException(400, { message: "missing_code_or_state" });
  }
  const pkce = await consumePkce(c.env.SESSIONS, state);
  if (!pkce || pkce.provider !== "google") {
    throw new HTTPException(400, { message: "unknown_state" });
  }

  const tokenResp = await fetch(GOOGLE_TOKEN_URL, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      code,
      client_id: c.env.GOOGLE_OAUTH_CLIENT_ID,
      client_secret: c.env.GOOGLE_OAUTH_CLIENT_SECRET,
      redirect_uri: `${c.env.JWT_ISSUER}/auth/google/callback`,
      grant_type: "authorization_code",
      code_verifier: pkce.code_verifier,
    }),
  });
  if (!tokenResp.ok) {
    const detail = await tokenResp.text();
    console.error("google token exchange failed", detail);
    throw new HTTPException(401, { message: "token_exchange_failed" });
  }
  const tokens = await tokenResp.json() as { access_token: string; id_token?: string };

  const userResp = await fetch(GOOGLE_USERINFO_URL, {
    headers: { authorization: `Bearer ${tokens.access_token}` },
  });
  if (!userResp.ok) {
    throw new HTTPException(401, { message: "userinfo_failed" });
  }
  const user = await userResp.json() as {
    sub: string;
    email: string;
    email_verified: boolean;
    name?: string;
    picture?: string;
  };

  if (!user.email_verified) {
    throw new HTTPException(403, { message: "email_not_verified" });
  }
  const domain = user.email.split("@")[1]?.toLowerCase() ?? "";
  if (PERSONAL_EMAIL_DOMAINS.has(domain)) {
    throw new HTTPException(403, {
      message: "personal_email_blocked",
    });
  }

  const userId = await upsertUser(c.env.DB, {
    provider: "google",
    subject: user.sub,
    email: user.email,
    email_domain: domain,
    display_name: user.name ?? null,
    avatar_url: user.picture ?? null,
  });

  const session = await signSession(c.env, {
    sub: userId,
    email: user.email,
    email_domain: domain,
  });

  // Redirect back to the Tauri app via the attune:// deep link with
  // the session token attached. The app's deep-link handler reads it
  // from the URL fragment (not query — fragments aren't logged) and
  // stores it in Keychain.
  const referralSuffix = pkce.referral_token
    ? `&ref=${encodeURIComponent(pkce.referral_token)}`
    : "";
  const deepLink =
    `${c.env.APP_DEEP_LINK_SCHEME}://oauth/callback#token=${session}${referralSuffix}`;
  return c.redirect(deepLink, 302);
});

// ------------------------------------------------------------------
// Microsoft (skeleton — fill in the Graph token + userinfo URLs)
// ------------------------------------------------------------------
authRoutes.post(
  "/microsoft/start",
  zValidator("json", startSchema),
  async (c) => {
    const body = c.req.valid("json");
    const pkce = await beginPkce(c.env.SESSIONS, "microsoft", body);
    const url = new URL("https://login.microsoftonline.com/common/oauth2/v2.0/authorize");
    url.searchParams.set("client_id", c.env.MICROSOFT_OAUTH_CLIENT_ID);
    url.searchParams.set("response_type", "code");
    url.searchParams.set("scope", "openid email profile User.Read offline_access");
    url.searchParams.set("redirect_uri", `${c.env.JWT_ISSUER}/auth/microsoft/callback`);
    url.searchParams.set("state", pkce.state);
    url.searchParams.set("code_challenge", pkce.code_challenge);
    url.searchParams.set("code_challenge_method", pkce.code_challenge_method);
    return c.json({ auth_url: url.toString(), state: pkce.state });
  },
);

authRoutes.get("/microsoft/callback", async (_c) => {
  // TODO: parallel structure to /google/callback. Exchange code for
  // tokens at https://login.microsoftonline.com/common/oauth2/v2.0/token,
  // fetch userinfo from https://graph.microsoft.com/v1.0/me, upsert
  // user, sign session, deep-link redirect. Same personal-email gate.
  throw new HTTPException(501, { message: "not_implemented" });
});

// ------------------------------------------------------------------
// Logout
// ------------------------------------------------------------------
authRoutes.post("/logout", requireAuth, async (c) => {
  const header = c.req.header("Authorization")!;
  const token = header.slice("Bearer ".length).trim();
  const claims = await verifySession(c.env, token);
  const remainingSeconds = Math.max(0, claims.exp - Math.floor(Date.now() / 1000));
  await revokeSession(c.env, claims.jti, remainingSeconds);
  return c.json({ ok: true });
});

// ------------------------------------------------------------------
// Helpers
// ------------------------------------------------------------------

async function upsertUser(
  db: D1Database,
  input: {
    provider: "google" | "microsoft" | "sso";
    subject: string;
    email: string;
    email_domain: string;
    display_name: string | null;
    avatar_url: string | null;
  },
): Promise<string> {
  // Look up by (provider, subject) first — handles re-login.
  const existing = await db
    .prepare(
      `SELECT id FROM users
        WHERE oauth_provider = ?1 AND oauth_subject = ?2
        AND deleted_at IS NULL`,
    )
    .bind(input.provider, input.subject)
    .first<{ id: string }>();
  if (existing) {
    await db
      .prepare(
        `UPDATE users SET
            email = ?2, email_domain = ?3, display_name = ?4, avatar_url = ?5,
            updated_at = datetime('now')
          WHERE id = ?1`,
      )
      .bind(existing.id, input.email, input.email_domain, input.display_name, input.avatar_url)
      .run();
    return existing.id;
  }
  // New user.
  const id = crypto.randomUUID();
  await db
    .prepare(
      `INSERT INTO users
         (id, email, email_domain, display_name, avatar_url, oauth_provider, oauth_subject)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)`,
    )
    .bind(id, input.email, input.email_domain, input.display_name, input.avatar_url, input.provider, input.subject)
    .run();
  return id;
}
