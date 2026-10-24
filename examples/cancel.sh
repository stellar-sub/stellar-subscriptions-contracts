#!/usr/bin/env bash
# examples/cancel.sh
#
# Example: cancel a subscription as the demo subscriber.
#
# What this script expects to happen:
#   1. The demo subscriber cancels their subscription on the subscription
#      contract. After this call the subscription is revoked.
#   2. A subsequent `charge` attempt by the merchant is rejected by the
#      contract with error #14 (REVOCATION), because a cancelled
#      subscription can no longer be charged.
#
# Usage:
#   ./examples/cancel.sh
#
# Prerequisites:
#   - The contract must already be deployed and initialised on Testnet
#     (see deploy.sh / verify-testnet.sh).
#   - The demo plan must exist and the demo subscriber must be subscribed
#     (run examples/create-plan.sh and examples/subscribe.sh first).
#   - The demo identities must be funded on Testnet.

set -euo pipefail

# Resolve the examples directory so the script works from any CWD.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=env.sh
source "${SCRIPT_DIR}/env.sh"

echo "==> Cancelling subscription"
echo "    Contract:   ${SUBSCRIPTION_CONTRACT_ID}"
echo "    Subscriber: ${DEMO_SUBSCRIBER}"
echo "    Plan:       ${DEMO_PLAN_ID}"
echo
echo "Expected: the subscription is revoked and the contract emits a"
echo "          cancellation event. No further charges will be accepted."
echo

stellar contract invoke \
  --id "${SUBSCRIPTION_CONTRACT_ID}" \
  --source "${DEMO_SUBSCRIBER}" \
  --network "${NETWORK}" \
  -- \
  cancel \
  --subscriber "${DEMO_SUBSCRIBER}" \
  --plan_id "${DEMO_PLAN_ID}"

echo
echo "==> Subscription cancelled."
echo
echo "==> Verifying that a charge after cancel is rejected"
echo "    Attempting to charge the cancelled subscription as the merchant."
echo
echo "Expected: the contract rejects the charge with error #14 (REVOCATION)."
echo

set +e
CHARGE_OUTPUT="$(
  stellar contract invoke \
    --id "${SUBSCRIPTION_CONTRACT_ID}" \
    --source "${DEMO_MERCHANT}" \
    --network "${NETWORK}" \
    -- \
    charge \
    --merchant "${DEMO_MERCHANT}" \
    --subscriber "${DEMO_SUBSCRIBER}" \
    --plan_id "${DEMO_PLAN_ID}" 2>&1
)"
CHARGE_STATUS=$?
set -e

echo "${CHARGE_OUTPUT}"
echo

if [[ ${CHARGE_STATUS} -eq 0 ]]; then
  echo "ERROR: charge unexpectedly succeeded after cancel." >&2
  echo "       Expected rejection with error #14 (REVOCATION)." >&2
  exit 1
fi

if grep -qE '(^|[^0-9])14([^0-9]|$)|REVOCATION' <<<"${CHARGE_OUTPUT}"; then
  echo "OK: charge was rejected with error #14 (REVOCATION), as expected."
else
  echo "WARNING: charge failed, but the output did not clearly show" >&2
  echo "         error #14 (REVOCATION). Inspect the output above." >&2
  exit 1
fi