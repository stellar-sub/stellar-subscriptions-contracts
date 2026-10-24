#!/usr/bin/env bash
#
# examples/charge.sh
#
# Charge a subscription for the current interval.
#
# What this script does:
#   1. Loads the demo identities and contract IDs from examples/env.sh.
#   2. Calls `charge` on the subscription contract as the demo merchant.
#   3. Prints the expected outcome, then attempts a second charge in the
#      same interval to show that it is rejected with error #11
#      (IntervalNotElapsed).
#
# Expected outcome:
#   - The first `charge` succeeds and transfers the plan amount from the
#     subscriber to the merchant.
#   - The second `charge` fails with error #11 because the interval has
#     not elapsed yet.
#
# Prerequisites:
#   - Run examples/create-plan.sh and examples/subscribe.sh first.
#   - A funded demo merchant and subscriber (see examples/README.md).
#
# Usage:
#   ./examples/charge.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=examples/env.sh
source "${SCRIPT_DIR}/env.sh"

echo "==> Charging subscription ${SUBSCRIPTION_ID} as merchant ${MERCHANT}"
echo "    Expect: charge succeeds and moves ${PLAN_AMOUNT} from subscriber to merchant."

stellar contract invoke \
  --id "${SUBSCRIPTION_CONTRACT_ID}" \
  --source "${MERCHANT}" \
  --network "${NETWORK}" \
  -- \
  charge \
  --subscription_id "${SUBSCRIPTION_ID}"

echo
echo "==> First charge succeeded."
echo
echo "==> Attempting a second charge in the same interval."
echo "    Expect: rejected with error #11 (IntervalNotElapsed)."

set +e
SECOND_CHARGE_OUTPUT="$(
  stellar contract invoke \
    --id "${SUBSCRIPTION_CONTRACT_ID}" \
    --source "${MERCHANT}" \
    --network "${NETWORK}" \
    -- \
    charge \
    --subscription_id "${SUBSCRIPTION_ID}" 2>&1
)"
SECOND_CHARGE_STATUS=$?
set -e

echo "${SECOND_CHARGE_OUTPUT}"

if [[ ${SECOND_CHARGE_STATUS} -eq 0 ]]; then
  echo
  echo "!! Unexpected: the second charge succeeded." >&2
  echo "   The interval should not have elapsed yet; check the plan interval." >&2
  exit 1
fi

if [[ "${SECOND_CHARGE_OUTPUT}" != *"#11"* && "${SECOND_CHARGE_OUTPUT}" != *"IntervalNotElapsed"* ]]; then
  echo
  echo "!! Unexpected: the second charge failed, but not with error #11." >&2
  echo "   Review the output above." >&2
  exit 1
fi

echo
echo "==> Second charge correctly rejected with error #11 (IntervalNotElapsed)."
echo "    Done."