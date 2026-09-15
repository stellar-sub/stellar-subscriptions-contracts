#!/usr/bin/env bash
#
# Deploy the plan, registry and subscription contracts to a Soroban network
# (Testnet by default), initialize them, and link them together.
#
# Usage:
#   ./scripts/deploy.sh
#   NETWORK=testnet IDENTITY=subs-deployer ./scripts/deploy.sh
#   DEMO=1 ./scripts/deploy.sh      # also run a live plan -> subscribe -> charge
#
# Requirements: the stellar CLI and, for Testnet, internet access to fund new
# identities through friendbot.
#
# Linking is one-shot by design: each contract accepts its links exactly
# once and only before its first plan or subscription. So this script always
# deploys fresh instances and links them before anything else touches them.

set -euo pipefail

NETWORK="${NETWORK:-testnet}"
IDENTITY="${IDENTITY:-subs-deployer}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_DIR="$ROOT/target/wasm32v1-none/release"
OUT_FILE="$ROOT/.deployments/$NETWORK.env"

command -v stellar >/dev/null || {
  echo "error: the stellar CLI is required (https://developers.stellar.org/docs/tools/cli)" >&2
  exit 1
}

ensure_identity() {
  local name="$1"
  if ! stellar keys address "$name" >/dev/null 2>&1; then
    echo "==> Generating and funding identity '$name' on $NETWORK..."
    stellar keys generate "$name" --network "$NETWORK" --fund >/dev/null
  fi
  stellar keys address "$name"
}

deploy() {
  # deploy <wasm-name> -> prints the new contract id
  stellar contract deploy \
    --wasm "$WASM_DIR/$1.wasm" \
    --source-account "$IDENTITY" \
    --network "$NETWORK"
}

invoke() {
  # invoke <source-identity> <contract-id> <fn> [args...]
  local source="$1" id="$2"
  shift 2
  stellar contract invoke \
    --id "$id" \
    --source-account "$source" \
    --network "$NETWORK" \
    -- "$@"
}

echo "==> Network:  $NETWORK"
echo "==> Identity: $IDENTITY"
ADMIN="$(ensure_identity "$IDENTITY")"
echo "==> Admin:    $ADMIN"

echo "==> Building contracts..."
(cd "$ROOT" && stellar contract build >/dev/null)

echo "==> Deploying..."
PLAN_ID="$(deploy plan)"
echo "    plan:         $PLAN_ID"
REGISTRY_ID="$(deploy registry)"
echo "    registry:     $REGISTRY_ID"
SUBSCRIPTION_ID="$(deploy subscription)"
echo "    subscription: $SUBSCRIPTION_ID"

echo "==> Initializing..."
invoke "$IDENTITY" "$PLAN_ID" initialize --admin "$ADMIN" >/dev/null
invoke "$IDENTITY" "$REGISTRY_ID" initialize --admin "$ADMIN" >/dev/null
invoke "$IDENTITY" "$SUBSCRIPTION_ID" initialize --admin "$ADMIN" >/dev/null

echo "==> Linking (one-shot, before any plan or subscription exists)..."
invoke "$IDENTITY" "$REGISTRY_ID" set_writers --admin "$ADMIN" \
  --subscription_contract "$SUBSCRIPTION_ID" --plan_contract "$PLAN_ID" >/dev/null
invoke "$IDENTITY" "$PLAN_ID" set_registry --admin "$ADMIN" --registry "$REGISTRY_ID" >/dev/null
invoke "$IDENTITY" "$SUBSCRIPTION_ID" set_plan_contract --admin "$ADMIN" --plan_contract "$PLAN_ID" >/dev/null
invoke "$IDENTITY" "$SUBSCRIPTION_ID" set_registry --admin "$ADMIN" --registry "$REGISTRY_ID" >/dev/null

mkdir -p "$(dirname "$OUT_FILE")"
cat >"$OUT_FILE" <<EOF
NETWORK=$NETWORK
ADMIN=$ADMIN
PLAN_CONTRACT_ID=$PLAN_ID
REGISTRY_CONTRACT_ID=$REGISTRY_ID
SUBSCRIPTION_CONTRACT_ID=$SUBSCRIPTION_ID
EOF

if [[ "${DEMO:-0}" == "1" ]]; then
  echo "==> Demo: plan -> subscribe -> charge with native XLM..."
  XLM_ID="$(stellar contract id asset --asset native --network "$NETWORK")"
  MERCHANT="$(ensure_identity subs-demo-merchant)"
  SUBSCRIBER="$(ensure_identity subs-demo-subscriber)"

  # 1 XLM every 720 ledgers (about an hour), capped at 12 XLM.
  AMOUNT=10000000
  INTERVAL=720
  CAP=120000000

  DEMO_PLAN="$(invoke subs-demo-merchant "$PLAN_ID" create_plan \
    --merchant "$MERCHANT" --name "Demo hourly" --token "$XLM_ID" \
    --amount_per_period "$AMOUNT" --interval_ledgers "$INTERVAL" --default_cap "$CAP")"
  echo "    plan id:         $DEMO_PLAN"

  DEMO_SUB="$(invoke subs-demo-subscriber "$SUBSCRIPTION_ID" subscribe \
    --subscriber "$SUBSCRIBER" --merchant "$MERCHANT" --token "$XLM_ID" \
    --amount_per_period "$AMOUNT" --interval_ledgers "$INTERVAL" --total_cap "$CAP" \
    --plan_id "$DEMO_PLAN")"
  echo "    subscription id: $DEMO_SUB"

  invoke subs-demo-merchant "$SUBSCRIPTION_ID" charge \
    --merchant "$MERCHANT" --subscription_id "$DEMO_SUB" >/dev/null
  echo "    charged once"

  echo "    second charge in the same interval (must be rejected):"
  if invoke subs-demo-merchant "$SUBSCRIPTION_ID" charge \
    --merchant "$MERCHANT" --subscription_id "$DEMO_SUB" >/dev/null 2>&1; then
    echo "error: INTERVAL invariant violated on $NETWORK" >&2
    exit 1
  fi
  echo "    rejected, as required"

  echo "    registry stats:"
  invoke "$IDENTITY" "$REGISTRY_ID" get_stats

  cat >>"$OUT_FILE" <<EOF
DEMO_TOKEN=$XLM_ID
DEMO_MERCHANT=$MERCHANT
DEMO_SUBSCRIBER=$SUBSCRIBER
DEMO_PLAN_ID=$DEMO_PLAN
DEMO_SUBSCRIPTION_ID=$DEMO_SUB
EOF
fi

cat <<EOF

============================================================
  Deployment complete on $NETWORK
============================================================
  Admin:         $ADMIN
  plan:          $PLAN_ID
  registry:      $REGISTRY_ID
  subscription:  $SUBSCRIPTION_ID
============================================================
Contract ids written to ${OUT_FILE#"$ROOT"/}
Record them in DEPLOYMENTS.md.
EOF
