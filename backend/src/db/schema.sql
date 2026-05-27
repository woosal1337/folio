-- Attune backend schema (D1 / SQLite).
-- See obsidian.md/projects/attune/notes/ADR-001-backend-architecture.md
-- for the data-storage policy. Anything not in this schema is, by
-- design, not allowed to live on the server.

PRAGMA foreign_keys = ON;

-- ============================================================
-- Users
-- ============================================================
CREATE TABLE IF NOT EXISTS users (
  id              TEXT PRIMARY KEY,                  -- UUIDv7
  email           TEXT NOT NULL UNIQUE,
  email_domain    TEXT NOT NULL,                     -- denormalised for workspace auto-join lookup
  display_name    TEXT,
  avatar_url      TEXT,
  oauth_provider  TEXT NOT NULL CHECK (oauth_provider IN ('google', 'microsoft', 'sso', 'local')),
  oauth_subject   TEXT NOT NULL,                     -- provider's sub claim; nullable for local
  language        TEXT NOT NULL DEFAULT 'en',
  created_at      TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at      TEXT NOT NULL DEFAULT (datetime('now')),
  deleted_at      TEXT,                              -- GDPR Art. 17 soft delete
  UNIQUE (oauth_provider, oauth_subject)
);

CREATE INDEX IF NOT EXISTS idx_users_email_domain ON users (email_domain) WHERE deleted_at IS NULL;

-- ============================================================
-- Workspaces — sharing + billing primitive.
-- A user can own multiple workspaces; a workspace has many members.
-- ============================================================
CREATE TABLE IF NOT EXISTS workspaces (
  id                   TEXT PRIMARY KEY,
  name                 TEXT NOT NULL,
  slug                 TEXT NOT NULL UNIQUE,
  logo_url             TEXT,
  bucket               TEXT NOT NULL CHECK (bucket IN ('founder', 'clinical', 'sales', 'education')),
  owner_user_id        TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  domain               TEXT,                                  -- e.g. 'clinora.ai' for B2B auto-join
  allow_discovery      INTEGER NOT NULL DEFAULT 1,
  allow_auto_join      INTEGER NOT NULL DEFAULT 1,            -- set to 0 for Clinical bucket
  tier                 TEXT NOT NULL DEFAULT 'free' CHECK (tier IN ('free', 'pro', 'clinical', 'enterprise')),
  created_at           TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at           TEXT NOT NULL DEFAULT (datetime('now')),
  deleted_at           TEXT
);

CREATE INDEX IF NOT EXISTS idx_workspaces_domain ON workspaces (domain) WHERE deleted_at IS NULL AND allow_discovery = 1;

-- ============================================================
-- Workspace membership
-- ============================================================
CREATE TABLE IF NOT EXISTS workspace_members (
  workspace_id  TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  user_id       TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role          TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'member')),
  joined_at     TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (workspace_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_workspace_members_user ON workspace_members (user_id);

-- ============================================================
-- Invites (pending)
-- ============================================================
CREATE TABLE IF NOT EXISTS invites (
  id            TEXT PRIMARY KEY,
  workspace_id  TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  inviter_id    TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  email         TEXT NOT NULL,
  token         TEXT NOT NULL UNIQUE,
  role          TEXT NOT NULL DEFAULT 'member',
  expires_at    TEXT NOT NULL,
  accepted_at   TEXT,
  created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_invites_email ON invites (email) WHERE accepted_at IS NULL;

-- ============================================================
-- Clinical license verifications — gates the Clinical tier.
-- See GET-130. Verification proof is stored as a hash (not the raw
-- NPI / license number) to comply with HIPAA minimum-necessary
-- principle even though it's not PHI under the BAA-free architecture.
-- ============================================================
CREATE TABLE IF NOT EXISTS clinical_verifications (
  id              TEXT PRIMARY KEY,
  user_id         TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
  method          TEXT NOT NULL CHECK (method IN ('npi', 'state_license', 'institution_domain')),
  proof_hash      TEXT NOT NULL,                     -- SHA-256 of (method || value || salt)
  verified_at     TEXT NOT NULL DEFAULT (datetime('now')),
  verifier_log    TEXT                                 -- JSON audit trail (which API was called, response code)
);

-- ============================================================
-- Subscriptions — Stripe is the source of truth, this table mirrors.
-- ============================================================
CREATE TABLE IF NOT EXISTS subscriptions (
  id                       TEXT PRIMARY KEY,
  workspace_id             TEXT NOT NULL UNIQUE REFERENCES workspaces(id) ON DELETE CASCADE,
  stripe_customer_id       TEXT NOT NULL,
  stripe_subscription_id   TEXT,
  tier                     TEXT NOT NULL CHECK (tier IN ('pro', 'clinical', 'enterprise')),
  status                   TEXT NOT NULL CHECK (status IN ('trialing', 'active', 'past_due', 'canceled', 'unpaid')),
  current_period_end       TEXT,
  cancel_at_period_end     INTEGER NOT NULL DEFAULT 0,
  created_at               TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at               TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================
-- Referrals — see GET-140 + GET-141.
-- ============================================================
CREATE TABLE IF NOT EXISTS referral_codes (
  id          TEXT PRIMARY KEY,
  user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  token       TEXT NOT NULL UNIQUE,
  created_at  TEXT NOT NULL DEFAULT (datetime('now')),
  revoked_at  TEXT
);

CREATE TABLE IF NOT EXISTS referral_attributions (
  id                       TEXT PRIMARY KEY,
  code_id                  TEXT NOT NULL REFERENCES referral_codes(id) ON DELETE CASCADE,
  referee_user_id          TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
  signed_up_at             TEXT NOT NULL DEFAULT (datetime('now')),
  validated                INTEGER NOT NULL DEFAULT 0,
  validation_failure_code  TEXT,                      -- e.g. 'personal_email', 'duplicate_ip', 'fraud_flag'
  validated_at             TEXT,
  reward_granted_at        TEXT
);

CREATE INDEX IF NOT EXISTS idx_referral_attributions_code ON referral_attributions (code_id);

-- ============================================================
-- Audit log — every privacy-relevant event lands here.
-- Visible to the user in Settings → Workspace → Data security.
-- ============================================================
CREATE TABLE IF NOT EXISTS audit_log (
  id          TEXT PRIMARY KEY,
  user_id     TEXT REFERENCES users(id) ON DELETE SET NULL,
  workspace_id TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
  event       TEXT NOT NULL,                          -- e.g. 'oauth.login', 'workspace.created', 'mcp.token.issued'
  payload     TEXT,                                   -- JSON, minimal
  ip_hash     TEXT,                                   -- SHA-256 of the IP for fraud detection without storing raw IPs
  created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_audit_log_user_created ON audit_log (user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_audit_log_workspace_created ON audit_log (workspace_id, created_at);
