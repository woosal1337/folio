/**
 * /users — authenticated user profile.
 *
 * GET    /users/me   → current user's record (no workspaces — see /workspaces)
 * PATCH  /users/me   → update display_name / avatar_url / language
 * DELETE /users/me   → GDPR Art. 17 right-to-erasure (soft delete)
 */

import { Hono } from "hono";
import { HTTPException } from "hono/http-exception";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import type { Env } from "../lib/env";
import { requireAuth } from "../lib/auth-middleware";

export const userRoutes = new Hono<Env>();
userRoutes.use("*", requireAuth);

userRoutes.get("/me", async (c) => {
  const userId = c.get("user_id")!;
  const user = await c.env.DB
    .prepare(
      `SELECT id, email, email_domain, display_name, avatar_url,
              oauth_provider, language, created_at
         FROM users WHERE id = ?1 AND deleted_at IS NULL`,
    )
    .bind(userId)
    .first();
  if (!user) throw new HTTPException(404, { message: "user_not_found" });
  return c.json(user);
});

userRoutes.patch(
  "/me",
  zValidator(
    "json",
    z.object({
      display_name: z.string().max(120).optional(),
      avatar_url: z.string().url().optional(),
      language: z.string().regex(/^[a-z]{2}(-[A-Za-z]+)?$/).optional(),
    }),
  ),
  async (c) => {
    const userId = c.get("user_id")!;
    const body = c.req.valid("json");
    const setters: string[] = [];
    const binds: (string | null)[] = [];
    let i = 1;
    for (const [k, v] of Object.entries(body)) {
      if (v !== undefined) {
        setters.push(`${k} = ?${++i}`);
        binds.push(v);
      }
    }
    if (setters.length === 0) {
      throw new HTTPException(400, { message: "no_fields" });
    }
    await c.env.DB
      .prepare(
        `UPDATE users SET ${setters.join(", ")}, updated_at = datetime('now')
           WHERE id = ?1`,
      )
      .bind(userId, ...binds)
      .run();
    return c.json({ ok: true });
  },
);

userRoutes.delete("/me", async (c) => {
  const userId = c.get("user_id")!;
  // Soft delete: respect GDPR Art. 17 + leave audit-log breadcrumbs.
  // Cascading FK deletes will clear workspace_members + invites +
  // referral_codes the user owns.
  await c.env.DB
    .prepare(`UPDATE users SET deleted_at = datetime('now') WHERE id = ?1`)
    .bind(userId)
    .run();
  return c.json({ ok: true });
});
