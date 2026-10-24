#!/usr/bin/env bash
# Shared environment and helpers for the runnable Testnet CLI examples.
#
# Usage (from another example script):
#
#   source "$(dirname "${BASH_SOURCE[0]}")/env.sh"
#
# This file:
#   * selects a network (default: testnet) and sources `.deployments/$NETWORK.env`
#   * validates the demo identities and contract IDs used by the examples
#   * defines a shared `invoke` helper around the stellar CLI
#
# It mirrors the patterns used in scripts/deploy.sh and scripts/verify-testnet.sh.

set -euo pipefail

# ---------------------------------------------------------------------------
# Locate the repository root so examples work from any working directory.
# ---------------------------------------------------------------------------
EXAMPLES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${EXAMPLES_DIR}/.." && pwd)"

# ---------------------------------------------------------------------------
# Network selection (defaults to testnet).
# ---------------------------------------------------------------------------
NETWORK="${NETWORK:-testnet}"
export NETWORK

DEPLOYMENTS_FILE="${REPO_ROOT}/.deployments/${NETWORK}.env"

if [[ ! -f "${DEPLOYMENTS_FILE}" ]]; then
  echo "error: deployment file not found: ${DEPLOYMENTS_FILE}" >&2
  echo "       run scripts/deploy.sh for the '${NETWORK}' network first," >&2
  echo "       or set NETWORK to an existing deployment (e.g. NETWORK=testnet)." >&2
  exit 1
fi

# shellcheck disable=SC1090
source "${DEPLOYMENTS_FILE}"

# ---------------------------------------------------------------------------
# Demo variables required by the examples.
# ---------------------------------------------------------------------------
REQUIRED_VARS=(
  DEMO_TOKEN
  DEMO_MERCHANT
  DEMO_SUBSCRIBER
  PLAN_CONTRACT_ID
  SUBSCRIPTION_CONTRACT_ID
)

missing=()
for var in "${REQUIRED_VARS[@]}"; do
  if [[ -z "${!var:-}" ]]; then
    missing+=("${var}")
  fi
done

if (( ${#missing[@]} > 0 )); then
  echo "error: missing required variable(s) in ${DEPLOYMENTS_FILE}:" >&2
  for var in "${missing[@]}"; do
    echo "       - ${var}" >&2
  done
  echo "       see examples/README.md for the expected deployment variables." >&2
  exit 1
fi

export DEMO_TOKEN DEMO_MERCHANT DEMO_SUBSCRIBER PLAN_CONTRACT_ID SUBSCRIPTION_CONTRACT_ID

# ---------------------------------------------------------------------------
# Stellar CLI configuration.
# ---------------------------------------------------------------------------
# The source account used to sign invocations. Defaults to the demo subscriber
# but can be overridden per example (e.g. the merchant for plan creation).
SOURCE_ACCOUNT="${SOURCE_ACCOUNT:-${DEMO_SUBSCRIBER}}"
export SOURCE_ACCOUNT

# Network passphrase / RPC URL: prefer values from the deployment file, falling
# back to the public Testnet defaults.
case "${NETWORK}" in
  testnet)
    NETWORK_PASSPHRASE="${NETWORK_PASSPHRASE:-Test SDF Network ; September 2015}"
    RPC_URL="${RPC_URL:-https://soroban-testnet.stellar.org}"
    ;;
  futurenet)
    NETWORK_PASSPHRASE="${NETWORK_PASSPHRASE:-Test SDF Future Network ; October 2022}"
    RPC_URL="${RPC_URL:-https://rpc-futurenet.stellar.org}"
    ;;
  *)
    : "${NETWORK_PASSPHRASE:?NETWORK_PASSPHRASE must be set for network '${NETWORK}'}"
    : "${RPC_URL:?RPC_URL must be set for network '${NETWORK}'}"
    ;;
esac

export NETWORK_PASSPHRASE RPC_URL

if ! command -v stellar >/dev/null 2>&1; then
  echo "error: the 'stellar' CLI was not found on PATH." >&2
  echo "       install it from https://developers.stellar.org/docs/tools/developer-tools/cli/stellar-cli" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Helpers.
# ---------------------------------------------------------------------------

# Print a short banner describing what an example is about to do.
#
#   banner "Create a plan" "This will create a new subscription plan."
banner() {
  local title="$1"
  shift || true
  echo
  echo "=============================================================="
  echo " ${title}"
  echo "=============================================================="
  local line
  for line in "$@"; do
    echo " ${line}"
  done
  echo
}

# Print the current example context (network, contracts, identities).
print_context() {
  echo "network:                 ${NETWORK}"
  echo "rpc url:                 ${RPC_URL}"
  echo "plan contract:           ${PLAN_CONTRACT_ID}"
  echo "subscription contract:   ${SUBSCRIPTION_CONTRACT_ID}"
  echo "demo token:              ${DEMO_TOKEN}"
  echo "demo merchant:           ${DEMO_MERCHANT}"
  echo "demo subscriber:         ${DEMO_SUBSCRIBER}"
  echo "source account:          ${SOURCE_ACCOUNT}"
  echo
}

# Invoke a contract function via the stellar CLI.
#
#   invoke <contract-id> <function> [args...]
#
# Any additional arguments are passed through to `stellar contract invoke`.
# The source account defaults to $SOURCE_ACCOUNT and can be overridden by
# exporting SOURCE_ACCOUNT before calling.
invoke() {
  if (( $# < 2 )); then
    echo "usage: invoke <contract-id> <function> [args...]" >&2
    return 1
  fi

  local contract_id="$1"
  local function_name="$2"
  shift 2

  echo "> stellar contract invoke \\"
  echo "    --id ${contract_id} \\"
  echo "    --source ${SOURCE_ACCOUNT} \\"
  echo "    --network ${NETWORK} \\"
  echo "    -- \\"
  echo "    ${function_name} $*"

  stellar contract invoke \
    --id "${contract_id}" \
    --source "${SOURCE_ACCOUNT}" \
    --network "${NETWORK}" \
    --rpc-url "${RPC_URL}" \
    --network-passphrase "${NETWORK_PASSPHRASE}" \
    -- \
    "${function_name}" "$@"
}

# Invoke a function on the plan contract.
invoke_plan() {
  invoke "${PLAN_CONTRACT_ID}" "$@"
}

# Invoke a function on the subscription contract.
invoke_subscription() {
  invoke "${SUBSCRIPTION_CONTRACT_ID}" "$@"
}