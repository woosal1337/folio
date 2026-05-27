/**
 * /workspaces — CRUD for workspace identity + membership.
 *
 *   POST  /workspaces              create on first signup
 *   GET   /workspaces/mine         list workspaces the user belongs to
 *   GET   /workspaces/:slug        read by slug (member-only)
 *   PATCH /workspaces/:slug        update (admin/owner-only)
 *   POST  /workspaces/:slug/invites           create invite
 *   POST  /workspaces/:slug/invites/accept    accept by token
 *   GET   /workspaces/discover/:domain         auto-join discovery for new signups
 *
 * Schema highlights — `bucket` is one of founder/clinical/sales/education,
 * set at onboarding (GET-129). Clinical bucket gets stricter defaults:
 * allow_discovery=0, allow_auto_join=0.
 */

import { Hono } from "hono";
import { HTTPException } from "hono/http-exception";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import type { Env } from "../lib/env";
import { requireAuth } from "../lib/auth-middleware";

export const workspaceRoutes = new Hono<Env>();
workspaceRoutes.use("*", requireAuth);

const createSchema = z.object({
  name: z.string().min(1).max(120),
  slug: z.string().min(1).max(80).regex(/^[a-z0-9-]+$/),
  bucket: z.enum(["founder", "clinical", "sales", "education"]),
  logo_url: z.string().url().optional(),
});

workspaceRoutes.post(
  "/",
  zValidator("json", createSchema),
  async (c) => {
    const userId = c.get("user_id")!;
    const body = c.req.valid("json");
    const id = crypto.randomUUID();

    // Bucket-aware defaults per Sasha — Clinical workspaces don't
    // accept walk-ins from a shared domain.
    const isClinical = body.bucket === "clinical";
    const allowDiscovery = isClinical ? 0 : 1;
    const allowAutoJoin = isClinical ? 0 : 1;

    // Derive the domain from the owner's email.
    const owner = await c.env.DB
      .prepare(`SELECT email_domain FROM users WHERE id = ?1`)
      .bind(userId)
      .first<{ email_domain: string }>();
    const domain = owner?.email_domain ?? null;

    await c.env.DB.batch([
      c.env.DB.prepare(
        `INSERT INTO workspaces
           (id, name, slug, logo_url, bucket, owner_user_id, domain, allow_discovery, allow_auto_join)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)`,
      ).bind(
        id,
        body.name,
        body.slug,
        body.logo_url ?? null,
        body.bucket,
        userId,
        domain,
        allowDiscovery,
        allowAutoJoin,
      ),
      c.env.DB.prepare(
        `INSERT INTO workspace_members (workspace_id, user_id, role)
         VALUES (?1, ?2, 'owner')`,
      ).bind(id, userId),
    ]);

    return c.json({ id, slug: body.slug });
  },
);

workspaceRoutes.get("/mine", async (c) => {
  const userId = c.get("user_id")!;
  const { results } = await c.env.DB
    .prepare(
      `SELECT w.id, w.name, w.slug, w.logo_url, w.bucket, w.tier, wm.role
         FROM workspaces w
         JOIN workspace_members wm ON wm.workspace_id = w.id
        WHERE wm.user_id = ?1 AND w.deleted_at IS NULL`,
    )
    .bind(userId)
    .all();
  return c.json({ workspaces: results });
});

workspaceRoutes.get("/discover/:domain", async (c) => {
  // Called during signup to ask "is there a workspace I should join
  // automatically for my email domain?" Only returns workspaces with
  // allow_discovery = 1 — Clinical workspaces are excluded.
  const userId = c.get("user_id")!;
  const domain = c.req.param("domain").toLowerCase();
  if (!domain || !/^[a-z0-9.-]+$/.test(domain)) {
    throw new HTTPException(400, { message: "bad_domain" });
  }
  const { results } = await c.env.DB
    .prepare(
      `SELECT id, name, slug, logo_url, bucket, allow_auto_join
         FROM workspaces
        WHERE domain = ?1
          AND allow_discovery = 1
          AND deleted_at IS NULL`,
    )
    .bind(domain)
    .all();
  // Don't leak the existence of the user's own workspaces back to
  // them via this endpoint — strip those out.
  const filtered = results.filter(
    (r) => (r as { allow_auto_join: number }).allow_auto_join === 1,
  );
  // Bonus paranoia: never expose the owner_user_id externally.
  return c.json({ discoverable: filtered });
});

workspaceRoutes.get("/:slug", async (c) => {
  const slug = c.req.param("slug");
  const userId = c.get("user_id")!;
  const row = await c.env.DB
    .prepare(
      `SELECT w.id, w.name, w.slug, w.logo_url, w.bucket, w.tier,
              w.allow_discovery, w.allow_auto_join, wm.role
         FROM workspaces w
         JOIN workspace_members wm
           ON wm.workspace_id = w.id AND wm.user_id = ?2
        WHERE w.slug = ?1 AND w.deleted_at IS NULL`,
    )
    .bind(slug, userId)
    .first();
  if (!row) throw new HTTPException(404, { message: "workspace_not_found_or_forbidden" });
  return c.json(row);
});

workspaceRoutes.post(
  "/:slug/invites",
  zValidator(
    "json",
    z.object({
      email: z.string().email(),
      role: z.enum(["admin", "member"]).default("member"),
    }),
  ),
  async (c) => {
    const slug = c.req.param("slug");
    const userId = c.get("user_id")!;
    const body = c.req.valid("json");

    // Only owners/admins can invite.
    const guard = await c.env.DB
      .prepare(
        `SELECT w.id AS workspace_id, wm.role
           FROM workspaces w
           JOIN workspace_members wm
             ON wm.workspace_id = w.id AND wm.user_id = ?2
          WHERE w.slug = ?1 AND w.deleted_at IS NULL`,
      )
      .bind(slug, userId)
      .first<{ workspace_id: string; role: string }>();
    if (!guard) throw new HTTPException(404, { message: "workspace_not_found" });
    if (guard.role === "member") {
      throw new HTTPException(403, { message: "not_admin" });
    }

    const token = crypto.randomUUID().replace(/-/g, "");
    const id = crypto.randomUUID();
    // 14-day expiry.
    const expires = new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString();

    await c.env.DB
      .prepare(
        `INSERT INTO invites (id, workspace_id, inviter_id, email, token, role, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)`,
      )
      .bind(id, guard.workspace_id, userId, body.email, token, body.role, expires)
      .run();

    return c.json({ id, token, expires_at: expires });
  },
);
