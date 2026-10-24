#!/usr/bin/env bash
#
# examples/subscribe.sh
#
# Example: subscribe the demo subscriber to the plan created by
# examples/create-plan.sh on the current Testnet deployment.
#
# What this script expects to happen:
#   1. It loads the demo identities and contract IDs from examples/env.sh.
#   2. It calls `subscribe` on the subscription contract, passing the plan id
#      created by create-plan.sh, the demo subscriber identity, and the plan's
#      interval/token configuration.
#   3. On success the contract returns a new subscription id. The script prints
#      that id so you can use it with charge.sh, pause.sh and cancel.sh.
#
# Prerequisites:
#   - Run examples/create-plan.sh first and keep the plan id it prints
#     (or export PLAN_ID before running this script).
#   - The demo subscriber must be funded on Testnet (see examples/README.md).
#
# Usage:
#   ./examples/subscribe.sh
#   PLAN_ID=<plan-id> ./examples/subscribe.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=examples/env.sh
source "${SCRIPT_DIR}/env.sh"

# ---------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------

# The plan id is produced by create-plan.sh. Allow overriding via env var.
if [[ -z "${PLAN_ID:-}" ]]; then
  echo "error: PLAN_ID is not set." >&2
  echo "       Run ./examples/create-plan.sh first, then re-run with:" >&2
  echo "         PLAN_ID=<plan-id> ./examples/subscribe.sh" >&2
  exit 1
fi

# The demo subscriber identity (from env.sh). Overridable for experimentation.
SUBSCRIBER_IDENTITY="${SUBSCRIBER_IDENTITY:-${DEMO_SUBSCRIBER_IDENTITY:-demo-subscriber}}"

# ---------------------------------------------------------------------------
# What we expect to happen
# ---------------------------------------------------------------------------

cat <<EOF
==> subscribe example
    network:            ${NETWORK}
    rpc url:            ${RPC_URL}
    subscription:       ${SUBSCRIPTION_CONTRACT_ID}
    plan id:            ${PLAN_ID}
    subscriber identity:${SUBSCRIBER_IDENTITY}

    Expected outcome:
      - The contract validates the plan exists and is active.
      - The subscriber is registered against the plan for the plan's interval.
      - A new subscription id is returned and printed below.

EOF

# ---------------------------------------------------------------------------
# Invoke the contract
# ---------------------------------------------------------------------------

echo "==> calling subscribe..."

SUBSCRIBE_OUTPUT="$(
  stellar contract invoke \
    --id "${SUBSCRIPTION_CONTRACT_ID}" \
    --source "${SUBSCRIBER_IDENTITY}" \
    --network "${NETWORK}" \
    --rpc-url "${RPC_URL}" \
    --network-passphrase "${NETWORK_PASSPHRASE}" \
    -- \
    subscribe \
      --plan_id "${PLAN_ID}" \
      --subscriber "${SUBSCRIBER_IDENTITY}"
)"

# The invoke command prints the returned subscription id (a u64) on stdout.
SUBSCRIPTION_ID="$(echo "${SUBSCRIBE_OUTPUT}" | tail -n 1 | tr -d '[:space:]')"

if [[ -z "${SUBSCRIPTION_ID}" ]]; then
  echo "error: subscribe did not return a subscription id." >&2
  echo "       raw output: ${SUBSCRIBE_OUTPUT}" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------

cat <<EOF

==> subscribe succeeded
    new subscription id: ${SUBSCRIPTION_ID}

    Next steps:
      - Charge the subscription:  SUBSCRIPTION_ID=${SUBSCRIPTION_ID} ./examples/charge.sh
      - Pause the subscription:   SUBSCRIPTION_ID=${SUBSCRIPTION_ID} ./examples/pause.sh
      - Cancel the subscription:  SUBSCRIPTION_ID=${SUBSCRIPTION_ID} ./examples/cancel.sh

EOF