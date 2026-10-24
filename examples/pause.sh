#!/usr/bin/env bash
#
# examples/pause.sh
#
# Pauses a subscription using the demo subscriber identity, then shows that a
# charge attempt while paused is rejected with error #15 (SubscriptionPaused).
#
# Run the earlier examples first (see examples/README.md for the order):
#   ./examples/create-plan.sh
#   ./examples/subscribe.sh
#   ./examples/charge.sh
#
# Usage:
#   ./examples/pause.sh
#
# Environment (see examples/env.sh):
#   SUBSCRIPTION_CONTRACT_ID  - deployed subscription contract id
#   DEMO_SUBSCRIBER           - demo subscriber identity (Stellar secret key)
#   DEMO_MERCHANT             - demo merchant identity (Stellar secret key)
#   NETWORK                   - testnet | futurenet | local (default: testnet)
#   SOURCE_ACCOUNT            - optional funded source account for fees

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=examples/env.sh
source "${SCRIPT_DIR}/env.sh"

: "${SUBSCRIPTION_CONTRACT_ID:?SUBSCRIPTION_CONTRACT_ID must be set (see examples/env.sh)}"
: "${DEMO_SUBSCRIBER:?DEMO_SUBSCRIBER must be set (see examples/env.sh)}"
: "${DEMO_MERCHANT:?DEMO_MERCHANT must be set (see examples/env.sh)}"

NETWORK="${NETWORK:-testnet}"
SOURCE_ACCOUNT="${SOURCE_ACCOUNT:-${DEMO_SUBSCRIBER}}"

echo "==> Pausing subscription"
echo "    Contract:   ${SUBSCRIPTION_CONTRACT_ID}"
echo "    Subscriber: ${DEMO_SUBSCRIBER}"
echo "    Network:    ${NETWORK}"
echo
echo "Expected outcome:"
echo "  - The subscription is marked paused by the subscriber."
echo "  - The 'pause' call succeeds and returns the updated subscription state."
echo "  - A subsequent charge attempt by the merchant is rejected with error #15"
echo "    (SubscriptionPaused)."
echo

stellar contract invoke \
  --id "${SUBSCRIPTION_CONTRACT_ID}" \
  --source "${SOURCE_ACCOUNT}" \
  --network "${NETWORK}" \
  -- \
  pause \
  --subscriber "${DEMO_SUBSCRIBER}"

echo
echo "==> Subscription paused."
echo
echo "==> Attempting a charge while paused (expected to fail with error #15)"
echo
echo "Expected outcome:"
echo "  - The charge call is rejected with error #15 (SubscriptionPaused)."
echo "  - No funds are transferred from the subscriber to the merchant."
echo

set +e
CHARGE_OUTPUT="$(
  stellar contract invoke \
    --id "${SUBSCRIPTION_CONTRACT_ID}" \
    --source "${SOURCE_ACCOUNT}" \
    --network "${NETWORK}" \
    -- \
    charge \
    --merchant "${DEMO_MERCHANT}" \
    --subscriber "${DEMO_SUBSCRIBER}" 2>&1
)"
CHARGE_STATUS=$?
set -e

echo "${CHARGE_OUTPUT}"
echo

if [[ ${CHARGE_STATUS} -eq 0 ]]; then
  echo "WARNING: charge unexpectedly succeeded while the subscription was paused." >&2
  echo "         Expected rejection with error #15 (SubscriptionPaused)." >&2
  exit 1
fi

if grep -qE '(#15|error[^0-9]*15|SubscriptionPaused)' <<<"${CHARGE_OUTPUT}"; then
  echo "OK: charge was rejected with error #15 (SubscriptionPaused), as expected."
else
  echo "WARNING: charge failed, but the output did not clearly show error #15." >&2
  echo "         Inspect the output above to confirm the rejection reason." >&2
  exit 1
fi

echo
echo "==> Done. To resume charging, run ./examples/cancel.sh or resume the"
echo "    subscription according to your deployment's supported operations."