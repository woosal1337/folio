/**
 * /clinical — license verification for the BAA-free Clinical wedge.
 *
 *   POST /clinical/verify    — submit NPI / state license / institution domain
 *   GET  /clinical/me        — current user's verification status
 *
 * See GET-130 for the full design. Verification is one-of-three:
 *   1. NPI number — looked up against NPPES public registry.
 *   2. State license — looked up against per-state board API (where available).
 *   3. Institution domain — match against a curated allowlist.
 *
 * The proof stored on the server is a SHA-256 hash of (method || value || salt)
 * so the raw NPI/licence number does not persist.
 */

import { Hono } from "hono";
import { HTTPException } from "hono/http-exception";
import { z } from "zod";
import { zValidator } from "@hono/zod-validator";
import type { Env } from "../lib/env";
import { requireAuth } from "../lib/auth-middleware";

export const clinicalRoutes = new Hono<Env>();
clinicalRoutes.use("*", requireAuth);

// Seed allowlist — expand from a maintained CSV/JSON checked into the repo.
const INSTITUTION_ALLOWLIST = new Set([
  "stanford.edu",
  "stanfordhealthcare.org",
  "ccf.org",          // Cleveland Clinic
  "mayo.edu",
  "ucsfmedicalcenter.org",
  "nyu.edu",
  "harvard.edu",
  "hms.harvard.edu",
  // …extend as needed; see docs/clinical-institutions.csv
]);

const verifySchema = z.discriminatedUnion("method", [
  z.object({
    method: z.literal("npi"),
    npi: z.string().regex(/^\d{10}$/),
  }),
  z.object({
    method: z.literal("state_license"),
    state: z.string().length(2),
    license_number: z.string().min(1).max(50),
  }),
  z.object({
    method: z.literal("institution_domain"),
    domain: z.string().min(1).max(120),
  }),
]);

async function sha256Hex(input: string): Promise<string> {
  const buf = new TextEncoder().encode(input);
  const hash = await crypto.subtle.digest("SHA-256", buf);
  return [...new Uint8Array(hash)].map((b) => b.toString(16).padStart(2, "0")).join("");
}

clinicalRoutes.post(
  "/verify",
  zValidator("json", verifySchema),
  async (c) => {
    const userId = c.get("user_id")!;
    const body = c.req.valid("json");
    let ok = false;
    let verifierLog = "";

    switch (body.method) {
      case "npi": {
        const url = new URL(c.env.NPPES_API_BASE);
        url.searchParams.set("version", "2.1");
        url.searchParams.set("number", body.npi);
        const r = await fetch(url.toString());
        if (r.ok) {
          const data = await r.json() as { result_count: number };
          ok = data.result_count > 0;
          verifierLog = `npi_lookup:${data.result_count}`;
        }
        break;
      }
      case "state_license": {
        // TODO: route per-state to the right verifier. Many states
        // (CA, NY, TX) have public lookup endpoints; others require
        // scraping or human review queue. Stub returns false until
        // per-state adapters land.
        ok = false;
        verifierLog = `state_license:not_implemented:${body.state}`;
        break;
      }
      case "institution_domain": {
        const domain = body.domain.toLowerCase();
        ok = INSTITUTION_ALLOWLIST.has(domain);
        verifierLog = `institution_match:${ok}`;
        break;
      }
    }

    if (!ok) {
      throw new HTTPException(422, { message: "verification_failed" });
    }

    const salt = crypto.randomUUID();
    const rawValue =
      body.method === "npi"
        ? body.npi
        : body.method === "state_license"
        ? `${body.state}:${body.license_number}`
        : body.domain;
    const proofHash = await sha256Hex(`${body.method}|${rawValue}|${salt}`);

    await c.env.DB
      .prepare(
        `INSERT INTO clinical_verifications (id, user_id, method, proof_hash, verifier_log)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (user_id) DO UPDATE SET
           method = excluded.method,
           proof_hash = excluded.proof_hash,
           verified_at = datetime('now'),
           verifier_log = excluded.verifier_log`,
      )
      .bind(crypto.randomUUID(), userId, body.method, proofHash, verifierLog)
      .run();

    return c.json({ ok: true, method: body.method });
  },
);

clinicalRoutes.get("/me", async (c) => {
  const userId = c.get("user_id")!;
  const row = await c.env.DB
    .prepare(
      `SELECT method, verified_at FROM clinical_verifications WHERE user_id = ?1`,
    )
    .bind(userId)
    .first();
  return c.json({ verified: row !== null, ...row });
});
