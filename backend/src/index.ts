/**
 * Attune backend — Cloudflare Workers entrypoint.
 *
 * Local-first invariant: this Worker never reads or stores meeting
 * audio, transcripts, summaries, or Memory entries. See ADR-001 for
 * the full architecture rationale.
 *
 * Stack:
 *   - Hono (routing)
 *   - D1 (SQLite)
 *   - KV (sessions + OAuth state, 24h TTL)
 *   - jose (JWT signing/verification)
 *   - Stripe Checkout (hosted; we never touch card data)
 */

import { Hono } from "hono";
import { cors } from "hono/cors";
import { logger } from "hono/logger";
import { secureHeaders } from "hono/secure-headers";
import { HTTPException } from "hono/http-exception";

import type { Env } from "./lib/env";
import { authRoutes } from "./routes/auth";
import { userRoutes } from "./routes/users";
import { workspaceRoutes } from "./routes/workspaces";
import { clinicalRoutes } from "./routes/clinical";
import { billingRoutes } from "./routes/billing";
import { referralRoutes } from "./routes/referrals";

const app = new Hono<Env>();

// ---------------------------------------------------------------
// Global middleware
// ---------------------------------------------------------------
app.use("*", logger());
app.use("*", secureHeaders({
  // The Worker has no UI — relax frame ancestors but keep everything
  // else strict. The Tauri app does not load HTML from this origin;
  // these headers are defence-in-depth for any future browser-side
  // pages we ship under api.attune.app.
  contentSecurityPolicy: {
    defaultSrc: ["'self'"],
    scriptSrc: ["'self'"],
    styleSrc: ["'self'", "'unsafe-inline'"],
    imgSrc: ["'self'", "data:"],
    connectSrc: ["'self'"],
    frameAncestors: ["'none'"],
  },
}));
app.use("*", cors({
  // Tauri 2 apps make requests from `tauri://localhost` (production)
  // and `http://localhost:1420` (dev). Allow both.
  origin: (origin) => {
    if (!origin) return null;
    if (origin === "tauri://localhost") return origin;
    if (origin.startsWith("http://localhost:")) return origin;
    if (origin === "https://attune.app") return origin;
    return null;
  },
  credentials: true,
  allowMethods: ["GET", "POST", "PATCH", "DELETE", "OPTIONS"],
  allowHeaders: ["Authorization", "Content-Type"],
  maxAge: 600,
}));

// ---------------------------------------------------------------
// Health check
// ---------------------------------------------------------------
app.get("/", (c) => {
  return c.json({
    service: "attune-backend",
    version: "0.1.0",
    invariant: "We never see your meetings. ADR-001.",
  });
});

// ---------------------------------------------------------------
// Route mounts
// ---------------------------------------------------------------
app.route("/auth", authRoutes);
app.route("/users", userRoutes);
app.route("/workspaces", workspaceRoutes);
app.route("/clinical", clinicalRoutes);
app.route("/billing", billingRoutes);
app.route("/referrals", referralRoutes);

// ---------------------------------------------------------------
// Error handler
// ---------------------------------------------------------------
app.onError((err, c) => {
  if (err instanceof HTTPException) {
    return c.json(
      { error: err.message, status: err.status },
      err.status,
    );
  }
  // Unexpected — log to Cloudflare's tail, return a generic 500.
  // Critically: never echo the user's input back in the error body,
  // since that's the classic XSS-via-error vector.
  console.error("unhandled error", err);
  return c.json({ error: "internal_error", status: 500 }, 500);
});

app.notFound((c) => c.json({ error: "not_found" }, 404));

export default app;
