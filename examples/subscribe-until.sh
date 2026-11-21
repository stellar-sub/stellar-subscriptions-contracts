#!/usr/bin/env bash
#
# examples/subscribe-until.sh
#
# Example: subscribe the demo subscriber to the plan created by
# examples/create-plan.sh with a fixed 30-day expiry, after which the
# subscription can never be charged again (see docs/testing.md / expiry.rs
# and docs/design/52-expires-at.md).
#
# What this script expects to happen:
#   1. It loads the demo identities and contract IDs from examples/env.sh.
#   2. It computes an expiry ledger roughly 30 days out and calls
#      `subscribe_until` on the subscription contract.
#   3. On success the contract returns a new subscription id. The script
#      prints that id so you can use it with charge.sh, pause.sh and
#      cancel.sh.
#
# Prerequisites:
#   - Run examples/create-plan.sh first and keep the plan id it prints
#     (or export PLAN_ID before running this script).
#   - The demo subscriber must be funded on Testnet (see examples/README.md).
#
# Usage:
#   ./examples/subscribe-until.sh
#   PLAN_ID=<plan-id> ./examples/subscribe-until.sh
#   EXPIRES_AT_LEDGER=<ledger> PLAN_ID=<plan-id> ./examples/subscribe-until.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=examples/env.sh
source "${SCRIPT_DIR}/env.sh"

# ---------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------

if [[ -z "${PLAN_ID:-}" ]]; then
  echo "error: PLAN_ID is not set." >&2
  echo "       Run ./examples/create-plan.sh first, then re-run with:" >&2
  echo "         PLAN_ID=<plan-id> ./examples/subscribe-until.sh" >&2
  exit 1
fi

SUBSCRIBER_IDENTITY="${SUBSCRIBER_IDENTITY:-${DEMO_SUBSCRIBER_IDENTITY:-demo-subscriber}}"

# 30 days of ledgers at the Stellar network's ~5s average close time.
# 30 days * 24h * 60m * 60s / 5s = 518,400 ledgers.
EXPIRY_LEDGERS="${EXPIRY_LEDGERS:-518400}"

if [[ -z "${EXPIRES_AT_LEDGER:-}" ]]; then
  CURRENT_LEDGER="$(
    stellar contract invoke \
      --id "${SUBSCRIPTION_CONTRACT_ID}" \
      --source "${SUBSCRIBER_IDENTITY}" \
      --network "${NETWORK}" \
      --rpc-url "${RPC_URL}" \
      --network-passphrase "${NETWORK_PASSPHRASE}" \
      --send=no \
      -- \
      get_subscription --subscription_id 0 2>/dev/null | grep -o '"[0-9]*"' | head -1 | tr -d '"'
  )"
  CURRENT_LEDGER="${CURRENT_LEDGER:-0}"
  EXPIRES_AT_LEDGER=$((CURRENT_LEDGER + EXPIRY_LEDGERS))
fi

# ---------------------------------------------------------------------------
# What we expect to happen
# ---------------------------------------------------------------------------

cat <<EOF
==> subscribe-until example
    network:            ${NETWORK}
    rpc url:            ${RPC_URL}
    subscription:       ${SUBSCRIPTION_CONTRACT_ID}
    plan id:            ${PLAN_ID}
    subscriber identity:${SUBSCRIBER_IDENTITY}
    expires_at_ledger:  ${EXPIRES_AT_LEDGER} (~30 days out)

    Expected outcome:
      - The contract validates the plan exists and is active.
      - The subscriber is registered against the plan.
      - Once the ledger sequence passes expires_at_ledger, charge fails
        and the subscription can never be charged again.
      - A new subscription id is returned and printed below.

EOF

# ---------------------------------------------------------------------------
# Invoke the contract
# ---------------------------------------------------------------------------

echo "==> calling subscribe_until..."

SUBSCRIBE_OUTPUT="$(
  stellar contract invoke \
    --id "${SUBSCRIPTION_CONTRACT_ID}" \
    --source "${SUBSCRIBER_IDENTITY}" \
    --network "${NETWORK}" \
    --rpc-url "${RPC_URL}" \
    --network-passphrase "${NETWORK_PASSPHRASE}" \
    -- \
    subscribe_until \
      --plan_id "${PLAN_ID}" \
      --subscriber "${SUBSCRIBER_IDENTITY}" \
      --expires_at_ledger "${EXPIRES_AT_LEDGER}"
)"

SUBSCRIPTION_ID="$(echo "${SUBSCRIBE_OUTPUT}" | tail -n 1 | tr -d '[:space:]')"

if [[ -z "${SUBSCRIPTION_ID}" ]]; then
  echo "error: subscribe_until did not return a subscription id." >&2
  echo "       raw output: ${SUBSCRIBE_OUTPUT}" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------

cat <<EOF

==> subscribe-until succeeded
    new subscription id: ${SUBSCRIPTION_ID}
    expires_at_ledger:    ${EXPIRES_AT_LEDGER}

    Next steps:
      - Charge the subscription:  SUBSCRIPTION_ID=${SUBSCRIPTION_ID} ./examples/charge.sh
      - Pause the subscription:   SUBSCRIPTION_ID=${SUBSCRIPTION_ID} ./examples/pause.sh
      - Cancel the subscription:  SUBSCRIPTION_ID=${SUBSCRIPTION_ID} ./examples/cancel.sh

EOF
