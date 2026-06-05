#!/usr/bin/env bash
#
# Reject any commit that introduces a known telemetry / analytics SDK
# into the dependency tree. v2 roadmap finding R11.
#
# Folio's brand promise: "no telemetry, logs are local." Once a
# Mixpanel / PostHog / Sentry / etc. lands in the lock files it is
# very hard to extricate — packages bleed transitive dependencies
# that import the SDK lazily. The cheapest defence is a CI check that
# greps the locks before a dirty package can land.
#
# Exit non-zero on any hit. Run from the repo root.

set -euo pipefail

# Add to this list as new telemetry vendors appear. Keep entries
# lowercase + as the canonical package name (npm scope + name for JS,
# crate name for Rust).
FORBIDDEN=(
  # ---- analytics / product analytics ----
  "@amplitude/analytics-browser"
  "@amplitude/analytics-node"
  "amplitude-js"
  "@mixpanel/browser"
  "mixpanel"
  "mixpanel-browser"
  "@segment/analytics-next"
  "@segment/analytics-node"
  "analytics-node"
  "posthog-js"
  "posthog-node"
  "plausible-tracker"
  "@plausible/analytics"
  "@heap/analytics"
  "fullstory"
  "@datadog/browser-rum"
  "@datadog/browser-logs"
  "logrocket"
  "smartlook-client"

  # ---- crash / error reporting ----
  "@sentry/browser"
  "@sentry/electron"
  "@sentry/node"
  "@sentry/react"
  "@sentry/tauri"
  "sentry"
  "@bugsnag/js"
  "bugsnag"
  "rollbar"
  "@rollbar/react"

  # ---- Rust crates ----
  "sentry"
  "sentry-tracing"
  "sentry-anyhow"
  "sentry-log"
  "console-subscriber"
  "rollbar-rs"
  "honeycomb-tracing"
  "datadog-tracing"
)

LOCKS=(
  "bun.lock"
  "package-lock.json"
  "pnpm-lock.yaml"
  "yarn.lock"
  "Cargo.lock"
)

EXIT=0
echo "Checking lock files for forbidden telemetry packages..."
for pkg in "${FORBIDDEN[@]}"; do
  for lock in "${LOCKS[@]}"; do
    [ -f "$lock" ] || continue
    if grep -F -q "\"${pkg}\"" "$lock" 2>/dev/null \
      || grep -F -q "name = \"${pkg}\"" "$lock" 2>/dev/null; then
      echo "FAIL: '${pkg}' appears in ${lock}"
      echo "  Folio is local-first: no telemetry / analytics / crash reporters."
      echo "  See projects/folio/plan/v2-roadmap-multi-agent-consensus.md (finding R11)."
      EXIT=1
    fi
  done
done

if [ "$EXIT" -eq 0 ]; then
  echo "OK — no forbidden telemetry packages found in lock files."
fi
exit "$EXIT"
