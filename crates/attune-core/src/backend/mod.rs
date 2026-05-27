//! HTTP client for the attune-api backend at `attune.chele.bi`.
//!
//! Layout:
//!   * [`client`] — `BackendClient` struct + the bearer/refresh request layer.
//!   * [`tokens`] — Keychain-backed token storage (separate service from LLM keys).
//!   * [`types`] — wire types shared across endpoint modules.
//!   * [`auth`] — `/api/auth/*` (signup OTP, verify, refresh, logout).
//!   * [`account`] — `/api/account/*` (profile, devices, soft-delete).
//!   * [`referrals`] — `/api/referrals/*`.
//!   * [`settings_sync`] — `/api/settings/*` snapshot pull/push.
//!
//! Every endpoint returns a fully-typed Rust struct; the wire envelope
//! (`{ success, message, data, error }`) is unwrapped in [`client`].

pub mod account;
pub mod auth;
pub mod client;
pub mod referrals;
pub mod settings_sync;
pub mod tokens;
pub mod types;

pub use client::{BackendClient, BackendError};
pub use tokens::{AuthTokens, TokenStore, UserIdentity};
