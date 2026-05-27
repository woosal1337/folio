# Attune backend

> **Local-first invariant: this backend never sees user audio, transcripts, summaries, Memory entries, or any meeting content. It exists only for identity, workspaces, billing, license verification, and referrals.**
>
> See `obsidian.md/projects/attune/notes/ADR-001-backend-architecture.md` for the full architectural decision record.

## Stack

- **Runtime:** Cloudflare Workers (TypeScript)
- **Framework:** [Hono](https://hono.dev) — minimal, edge-friendly, type-safe
- **Database:** Cloudflare D1 (SQLite at the edge)
- **Sessions:** Cloudflare KV (ephemeral, 24h TTL)
- **Payments:** Stripe Checkout (hosted, never touches card data)
- **Domain:** `api.attune.app` (production), `localhost:8787` (dev via `wrangler dev`)

## What this backend knows about

| Stored | Why |
|---|---|
| `users` (id, email, oauth_provider, oauth_subject, created_at) | Identity for workspace membership |
| `workspaces` (id, name, slug, logo_url, bucket, owner_user_id) | Sharing + billing primitive |
| `workspace_members` (workspace × user × role) | Membership graph |
| `invites` (workspace × email × token × expires_at) | Pending invitations |
| `clinical_verifications` (user × method × verified_at) | Gates the Clinical tier (HIPAA-clean wedge) |
| `subscriptions` (workspace × stripe_subscription_id × tier) | Tier sync from Stripe webhooks |
| `referral_codes` (user × token) | Referral program |
| `referral_attributions` (code × referee × validated) | Anti-abuse + reward fulfilment |

## What this backend never sees

- Audio bytes
- Transcripts (any format)
- Meeting summaries / agent run outputs
- Memory entries
- Action items / Tasks board content
- Calendar event details (we only track which calendars the user connected, not what's in them)

## Local development

```sh
cd backend
bun install
bun run dev        # wrangler dev — runs at localhost:8787 with a local D1
bun run db:migrate # apply schema migrations to local D1
bun test           # vitest
```

The Mac app's auth client points at `http://localhost:8787` when built in dev mode, `https://api.attune.app` in release builds. Switch via the `VITE_ATTUNE_BACKEND_URL` env var.

## Deployment

```sh
bun run deploy     # wrangler deploy — pushes to api.attune.app
```

Secrets are stored via `wrangler secret put`:
- `JWT_SIGNING_KEY` — 256-bit random key for session JWTs
- `GOOGLE_OAUTH_CLIENT_ID` + `GOOGLE_OAUTH_CLIENT_SECRET`
- `MICROSOFT_OAUTH_CLIENT_ID` + `MICROSOFT_OAUTH_CLIENT_SECRET`
- `STRIPE_SECRET_KEY` + `STRIPE_WEBHOOK_SECRET`
- `NPPES_API_BASE` (default `https://npiregistry.cms.hhs.gov/api/`)

## Routes

| Method | Path | Purpose |
|---|---|---|
| POST | `/auth/google/start` | Issue PKCE challenge, return auth URL |
| GET  | `/auth/google/callback` | OAuth code exchange, set session, redirect to `attune://oauth/callback` |
| POST | `/auth/microsoft/start` | Same shape, Microsoft Graph |
| GET  | `/auth/microsoft/callback` | Same shape, Microsoft Graph |
| POST | `/auth/sso/start` | SAML init (Enterprise tier) |
| POST | `/auth/logout` | Invalidate session |
| GET  | `/users/me` | Current user profile |
| POST | `/workspaces` | Create workspace (signup) |
| GET  | `/workspaces/:slug` | Read workspace by slug |
| PATCH | `/workspaces/:slug` | Update workspace metadata |
| POST | `/workspaces/:slug/invites` | Create invite |
| POST | `/workspaces/:slug/members/:user_id` | Add member |
| POST | `/clinical/verify` | Submit license verification (NPI / state / institution domain) |
| POST | `/billing/checkout` | Create Stripe Checkout session |
| POST | `/billing/webhook` | Stripe webhook handler |
| POST | `/referrals/generate` | Issue user's referral token |
| GET  | `/referrals/redeem/:token` | Redirect to signup with attribution cookie |
| POST | `/referrals/validate` | Validate referral on signup completion |

All routes return JSON. All authenticated routes use a bearer JWT in `Authorization: Bearer <token>`. The JWT is signed with `JWT_SIGNING_KEY`, has a 7-day TTL, and is opaque to the client — the Tauri app just stores it in Keychain.

## Privacy posture

This backend cannot read user meetings. By design, by data model, by deployment topology. The marketing claim *"Attune's servers cannot read your meetings even if compelled by subpoena. We don't have them."* is true and load-bearing.

If we ever ship a feature that requires transmitting meeting content, it must be:
1. Explicitly user-initiated (the user actively pastes / shares)
2. Encrypted client-side before transmission (sealed-sender pattern, see consensus #24)
3. Surfaced to the user via the Privacy Tier Colour Band (consensus #5) with a byte-count pre-call
