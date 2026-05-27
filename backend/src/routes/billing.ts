/**
 * /billing — Stripe Checkout + webhook listener.
 *
 *   POST /billing/checkout    create a Stripe Checkout session for a workspace
 *   POST /billing/webhook     receive Stripe events → update subscriptions table
 *   GET  /billing/portal      create a Stripe Customer Portal link (manage card / cancel)
 *
 * We NEVER touch card data. Stripe Checkout hosts the entire payment
 * surface; we just create the session, redirect the user, and consume
 * the webhook on completion.
 */

import { Hono } from "hono";
import { HTTPException } from "hono/http-exception";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import Stripe from "stripe";
import type { Env } from "../lib/env";
import { requireAuth } from "../lib/auth-middleware";

export const billingRoutes = new Hono<Env>();

// TODO: replace these with the live Stripe price IDs once they exist.
const PRICE_IDS = {
  pro_monthly: "price_PRO_MONTHLY_PLACEHOLDER",
  pro_annual: "price_PRO_ANNUAL_PLACEHOLDER",
  clinical_monthly: "price_CLINICAL_MONTHLY_PLACEHOLDER",
  clinical_annual: "price_CLINICAL_ANNUAL_PLACEHOLDER",
} as const;

billingRoutes.post(
  "/checkout",
  requireAuth,
  zValidator(
    "json",
    z.object({
      workspace_slug: z.string(),
      price_key: z.enum(Object.keys(PRICE_IDS) as [keyof typeof PRICE_IDS]),
      success_url: z.string().url(),
      cancel_url: z.string().url(),
    }),
  ),
  async (c) => {
    const body = c.req.valid("json");
    const userId = c.get("user_id")!;

    const ws = await c.env.DB
      .prepare(
        `SELECT w.id, w.name FROM workspaces w
            JOIN workspace_members wm ON wm.workspace_id = w.id AND wm.user_id = ?2
          WHERE w.slug = ?1 AND wm.role IN ('owner', 'admin')
            AND w.deleted_at IS NULL`,
      )
      .bind(body.workspace_slug, userId)
      .first<{ id: string; name: string }>();
    if (!ws) throw new HTTPException(403, { message: "not_owner_or_admin" });

    const stripe = new Stripe(c.env.STRIPE_SECRET_KEY, {
      apiVersion: "2025-09-30.acacia" as Stripe.LatestApiVersion,
    });
    const session = await stripe.checkout.sessions.create({
      mode: "subscription",
      line_items: [{ price: PRICE_IDS[body.price_key], quantity: 1 }],
      success_url: body.success_url,
      cancel_url: body.cancel_url,
      client_reference_id: ws.id,
      metadata: {
        workspace_id: ws.id,
        workspace_name: ws.name,
        // We deliberately do NOT include user-identifying data beyond
        // what Stripe needs for billing; user email is read from the
        // checkout form, not from us.
      },
    });
    return c.json({ url: session.url });
  },
);

billingRoutes.post("/webhook", async (c) => {
  // Stripe sends raw bodies; we need the signature in the header.
  const sig = c.req.header("stripe-signature");
  if (!sig) throw new HTTPException(400, { message: "missing_signature" });

  const stripe = new Stripe(c.env.STRIPE_SECRET_KEY, {
    apiVersion: "2025-09-30.acacia" as Stripe.LatestApiVersion,
  });
  const raw = await c.req.raw.clone().text();

  let event: Stripe.Event;
  try {
    event = await stripe.webhooks.constructEventAsync(
      raw, sig, c.env.STRIPE_WEBHOOK_SECRET,
    );
  } catch (e) {
    console.error("webhook signature failed", e);
    throw new HTTPException(400, { message: "bad_signature" });
  }

  // Minimal handler. Expand to handle subscription.updated /
  // subscription.deleted / invoice.payment_failed events.
  switch (event.type) {
    case "checkout.session.completed": {
      const session = event.data.object as Stripe.Checkout.Session;
      const workspaceId = session.client_reference_id;
      if (!workspaceId) break;
      // Subscription tier inference is based on the price ID resolved
      // from the session line items; for the v1 stub we just mark the
      // workspace as active and let the cron / next page-load reconcile.
      // TODO: fetch session.subscription, derive tier from price ID.
      await c.env.DB
        .prepare(
          `INSERT INTO subscriptions
             (id, workspace_id, stripe_customer_id, stripe_subscription_id, tier, status)
           VALUES (?1, ?2, ?3, ?4, 'pro', 'active')
           ON CONFLICT (workspace_id) DO UPDATE SET
             stripe_subscription_id = excluded.stripe_subscription_id,
             status = 'active',
             updated_at = datetime('now')`,
        )
        .bind(
          crypto.randomUUID(),
          workspaceId,
          (session.customer as string) ?? "",
          (session.subscription as string) ?? "",
        )
        .run();
      break;
    }
    default:
      // TODO: handle subscription lifecycle events.
      break;
  }

  return c.json({ received: true });
});
