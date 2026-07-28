#!/usr/bin/env bash
set -uo pipefail
export GH_TOKEN=$(gh auth token --user valoryyaa-byte 2>/dev/null)
N="$1"; TITLE="${2:-PR #$1}"
cd "$(dirname "$0")"
git checkout -q main && git fetch -q origin main && git reset -q --hard origin/main
git fetch -q origin "pull/$N/head:pr$N" -f || { echo "FETCH_FAIL"; exit 3; }
if git merge --no-ff -m "Merge PR #$N: $TITLE" "pr$N" >/tmp/mrg_$N.out 2>&1; then
  echo "CLEAN_MERGE"; else
  echo "CONFLICT files:"; git diff --name-only --diff-filter=U; fi
