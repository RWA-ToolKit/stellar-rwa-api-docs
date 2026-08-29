#!/usr/bin/env bash
# Baseline load test for the Stellar RWA API's read endpoints.
#
# Uses `hey` (https://github.com/rakyll/hey) to fire a fixed load of
# requests at each read endpoint and report latency percentiles / RPS.
# Install hey with: go install github.com/rakyll/hey@latest
# (or grab a prebuilt binary from the project's release page).
#
# Usage:
#   ./scripts/load_test.sh [base_url] [asset_id]
#
# Defaults to a locally-running API on :8080 and asset id "1". Start the
# API first, e.g.:
#   cargo run --release
#
# See docs/load-testing.md for how to read the results and what the
# recorded baseline numbers were.

set -euo pipefail

BASE_URL="${1:-http://localhost:8080}"
ASSET_ID="${2:-1}"

REQUESTS="${REQUESTS:-2000}"
CONCURRENCY="${CONCURRENCY:-50}"

if ! command -v hey >/dev/null 2>&1; then
  echo "error: 'hey' is not installed. See https://github.com/rakyll/hey" >&2
  exit 1
fi

ENDPOINTS=(
  "/health"
  "/stats"
  "/assets"
  "/assets/${ASSET_ID}"
  "/assets/${ASSET_ID}/holders"
  "/assets/${ASSET_ID}/compliance"
  "/assets/${ASSET_ID}/dividends"
)

echo "Load testing ${BASE_URL} (n=${REQUESTS}, c=${CONCURRENCY})"
echo "=============================================================="

for path in "${ENDPOINTS[@]}"; do
  echo
  echo "--- ${path} ---"
  hey -n "${REQUESTS}" -c "${CONCURRENCY}" "${BASE_URL}${path}"
done
