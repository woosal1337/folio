#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

read_toml_version() { sed -n 's/^version = "\([^"]*\)".*/\1/p' "$1" | head -1; }
read_json_version() {
  python3 - "$1" <<'PY'
import json, sys
print(json.load(open(sys.argv[1])).get("version", ""))
PY
}

workspace="$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version = "\([^"]*\)".*/\1/p;}' Cargo.toml | head -1)"
pkg="$(read_json_version package.json)"
tauri="$(read_json_version src-tauri/tauri.conf.json)"

printf 'workspace.package : %s\n' "$workspace"
printf 'package.json      : %s\n' "$pkg"
printf 'tauri.conf.json   : %s\n' "$tauri"

fail=0
if [[ "$workspace" != "$pkg" ]]; then echo "MISMATCH: Cargo workspace ($workspace) != package.json ($pkg)"; fail=1; fi
if [[ "$workspace" != "$tauri" ]]; then echo "MISMATCH: Cargo workspace ($workspace) != tauri.conf.json ($tauri)"; fail=1; fi

if ! grep -q "## \[$workspace\]" CHANGELOG.md; then
  echo "MISMATCH: CHANGELOG.md has no '## [$workspace]' section"
  fail=1
fi

if [[ "$fail" -ne 0 ]]; then
  echo "Version drift detected. Bump Cargo.toml [workspace.package], package.json, and tauri.conf.json together."
  exit 1
fi
echo "OK: all versions agree at $workspace"
