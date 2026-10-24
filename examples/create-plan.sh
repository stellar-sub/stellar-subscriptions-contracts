#!/usr/bin/env bash
#
# examples/create-plan.sh
#
# Creates a subscription plan on the Testnet plan contract using the demo
# merchant identity.
#
# Expected outcome:
#   - The plan contract stores a new plan owned by the demo merchant.
#   - The plan charges 1 XLM (10_000_000 stroops) every 720 ledgers.
#   - The plan has a default per-subscription cap of 12 XLM (120_000_000 stroops).
#   - The script prints the new plan id, which you will pass to
#     examples/subscribe.sh.
#
# Usage:
#   ./examples/create-plan.sh
#
# Prerequisites:
#   - examples/env.sh is configured (see examples/README.md).
#   - The demo merchant identity exists and is funded on Testnet.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=examples/env.sh
source "${SCRIPT_DIR}/env.sh"

# Plan parameters: 1 XLM per 720 ledgers, 12 XLM default cap.
AMOUNT=10000000        # 1 XLM in stroops
INTERVAL=720           # ledgers between charges
CAP=120000000          # 12 XLM in stroops

echo "Creating plan with the demo merchant identity '${MERCHANT_IDENTITY}'..."
echo "  amount:   ${AMOUNT} stroops (1 XLM)"
echo "  interval: ${INTERVAL} ledgers"
echo "  cap:      ${CAP} stroops (12 XLM)"
echo
echo "Expected outcome: a new plan is stored on the plan contract and its id"
echo "is printed below. Use that id with examples/subscribe.sh."
echo

PLAN_ID="$(
  stellar contract invoke \
    --id "${PLAN_CONTRACT_ID}" \
    --source "${MERCHANT_IDENTITY}" \
    --network "${NETWORK}" \
    -- create_plan \
    --merchant "${MERCHANT_ADDRESS}" \
    --amount "${AMOUNT}" \
    --interval "${INTERVAL}" \
    --cap "${CAP}"
)"

echo "Plan created."
echo "PLAN_ID=${PLAN_ID}"
echo
echo "Next: ./examples/subscribe.sh ${PLAN_ID}"