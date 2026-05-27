/**
 * /referrals — referral program (GET-140 frontend + GET-141 backend).
 *
 *   POST /referrals/generate          → issue user's personal token
 *   GET  /referrals/redeem/:token     → public redirect to signup
 *   POST /referrals/validate          → called on signup completion
 *   GET  /referrals/me                → list of own attribution records (anonymised)
 */

import { Hono } from "hono";
import { HTTPException } from "hono/http-exception";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import type { Env } from "../lib/env";
import { requireAuth } from "../lib/auth-middleware";

export const referralRoutes = new Hono<Env>();

referralRoutes.post("/generate", requireAuth, async (c) => {
  const userId = c.get("user_id")!;
  // One active code per user — return existing if present.
  const existing = await c.env.DB
    .prepare(
      `SELECT token FROM referral_codes WHERE user_id = ?1 AND revoked_at IS NULL`,
    )
    .bind(userId)
    .first<{ token: string }>();
  if (existing) return c.json({ token: existing.token });

  const token = crypto.randomUUID().replace(/-/g, "").slice(0, 10);
  await c.env.DB
    .prepare(
      `INSERT INTO referral_codes (id, user_id, token) VALUES (?1, ?2, ?3)`,
    )
    .bind(crypto.randomUUID(), userId, token)
    .run();
  return c.json({ token });
});

referralRoutes.get("/redeem/:token", async (c) => {
  const token = c.req.param("token");
  // Public endpoint — used by the marketing landing at join.attune.app
  // to redirect into the deep-link signup if the app is installed,
  // or to the website signup form otherwise. For now we just return
  // a JSON descriptor; the website handles user-agent sniffing.
  const code = await c.env.DB
    .prepare(
      `SELECT id FROM referral_codes WHERE token = ?1 AND revoked_at IS NULL`,
    )
    .bind(token)
    .first();
  if (!code) throw new HTTPException(404, { message: "invalid_token" });
  return c.json({
    valid: true,
    deep_link: `${c.env.APP_DEEP_LINK_SCHEME}://onboarding?ref=${token}`,
  });
});

referralRoutes.post(
  "/validate",
  requireAuth,
  zValidator("json", z.object({ token: z.string() })),
  async (c) => {
    const refereeId = c.get("user_id")!;
    const { token } = c.req.valid("json");

    const code = await c.env.DB
      .prepare(
        `SELECT id, user_id FROM referral_codes
            WHERE token = ?1 AND revoked_at IS NULL`,
      )
      .bind(token)
      .first<{ id: string; user_id: string }>();
    if (!code) throw new HTTPException(404, { message: "invalid_token" });

    if (code.user_id === refereeId) {
      throw new HTTPException(400, { message: "self_referral_blocked" });
    }
    // TODO: anti-abuse — same IP, same payment method, same email domain
    // root, disposable-email blocklist, >10/day from one referrer.
    // For v1 we record the attribution and let a background cron
    // re-validate before granting the reward.

    await c.env.DB
      .prepare(
        `INSERT INTO referral_attributions (id, code_id, referee_user_id, validated)
         VALUES (?1, ?2, ?3, 0)
         ON CONFLICT (referee_user_id) DO NOTHING`,
      )
      .bind(crypto.randomUUID(), code.id, refereeId)
      .run();

    return c.json({ recorded: true });
  },
);

referralRoutes.get("/me", requireAuth, async (c) => {
  const userId = c.get("user_id")!;
  const { results } = await c.env.DB
    .prepare(
      `SELECT a.signed_up_at, a.validated, a.reward_granted_at
         FROM referral_attributions a
         JOIN referral_codes c ON c.id = a.code_id
        WHERE c.user_id = ?1
        ORDER BY a.signed_up_at DESC
        LIMIT 100`,
    )
    .bind(userId)
    .all();
  // Anonymised on purpose: we never reveal who the referee was.
  return c.json({ count: results.length, recent: results });
});
