/**
 * Cloudflare Workers env binding shape. See wrangler.toml for the
 * source of truth. Anything added here must also be bound there.
 */

export type Bindings = {
  // Cloudflare bindings
  DB: D1Database;
  SESSIONS: KVNamespace;

  // Public vars (non-secret)
  NPPES_API_BASE: string;
  APP_DEEP_LINK_SCHEME: string;
  JWT_ISSUER: string;
  JWT_AUDIENCE: string;
  SESSION_TTL_DAYS: string;

  // Secrets (set via `wrangler secret put`)
  JWT_SIGNING_KEY: string;
  GOOGLE_OAUTH_CLIENT_ID: string;
  GOOGLE_OAUTH_CLIENT_SECRET: string;
  MICROSOFT_OAUTH_CLIENT_ID: string;
  MICROSOFT_OAUTH_CLIENT_SECRET: string;
  STRIPE_SECRET_KEY: string;
  STRIPE_WEBHOOK_SECRET: string;
};

export type Variables = {
  // Set by the auth middleware after a successful Bearer validation
  user_id?: string;
  workspace_id?: string;
};

export type Env = {
  Bindings: Bindings;
  Variables: Variables;
};
