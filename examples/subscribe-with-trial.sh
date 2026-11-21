#!/usr/bin/env bash
#
# examples/subscribe-with-trial.sh
#
# Example: subscribe the demo subscriber to the plan created by
# examples/create-plan.sh with a 1-week free trial before the first charge.
#
# What this script expects to happen:
#   1. It loads the demo identities and contract IDs from examples/env.sh.
#   2. It calls `subscribe` on the subscription contract with
#      trial_ledgers set so the first charge is not due until the trial
#      ends (~1 week of ledgers at ~5s/ledger).
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
#   ./examples/subscribe-with-trial.sh
#   PLAN_ID=<plan-id> ./examples/subscribe-with-trial.sh

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
  echo "         PLAN_ID=<plan-id> ./examples/subscribe-with-trial.sh" >&2
  exit 1
fi

SUBSCRIBER_IDENTITY="${SUBSCRIBER_IDENTITY:-${DEMO_SUBSCRIBER_IDENTITY:-demo-subscriber}}"

# ~1 week of ledgers at the Stellar network's ~5s average close time.
# 7 days * 24h * 60m * 60s / 5s = 120,960 ledgers.
TRIAL_LEDGERS="${TRIAL_LEDGERS:-120960}"

# ---------------------------------------------------------------------------
# What we expect to happen
# ---------------------------------------------------------------------------

cat <<EOF
==> subscribe-with-trial example
    network:            ${NETWORK}
    rpc url:            ${RPC_URL}
    subscription:       ${SUBSCRIPTION_CONTRACT_ID}
    plan id:            ${PLAN_ID}
    subscriber identity:${SUBSCRIBER_IDENTITY}
    trial_ledgers:      ${TRIAL_LEDGERS} (~1 week)

    Expected outcome:
      - The contract validates the plan exists and is active.
      - The subscriber is registered against the plan.
      - The first charge is not due until trial_ledgers have elapsed.
      - A new subscription id is returned and printed below.

EOF

# ---------------------------------------------------------------------------
# Invoke the contract
# ---------------------------------------------------------------------------

echo "==> calling subscribe with a trial period..."

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
      --subscriber "${SUBSCRIBER_IDENTITY}" \
      --trial_ledgers "${TRIAL_LEDGERS}"
)"

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

==> subscribe-with-trial succeeded
    new subscription id: ${SUBSCRIPTION_ID}
    trial_ledgers:        ${TRIAL_LEDGERS}

    Next steps:
      - Charge the subscription (will fail until the trial ends):
          SUBSCRIPTION_ID=${SUBSCRIPTION_ID} ./examples/charge.sh
      - Pause the subscription:   SUBSCRIPTION_ID=${SUBSCRIPTION_ID} ./examples/pause.sh
      - Cancel the subscription:  SUBSCRIPTION_ID=${SUBSCRIPTION_ID} ./examples/cancel.sh

EOF
