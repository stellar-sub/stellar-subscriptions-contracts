#!/usr/bin/env bash
#
# Prove CAP, INTERVAL and REVOCATION against a live deployment.
#
# Creates fresh subscriptions with the demo identities that deploy.sh made
# (DEMO=1), then tries to break each invariant and checks the contract says no
# with the right error code:
#
#   INTERVAL    a second charge in the same interval           -> #11
#   pause       a charge while paused                          -> #15
#   REVOCATION  a charge, pause, resume or cancel after cancel -> #14
#   CAP         a charge that would pass the cap               -> #12
#
# Usage:  ./scripts/verify-testnet.sh        (after DEMO=1 ./scripts/deploy.sh)
#
# A rejected call is caught when the CLI simulates it, so nothing is
# submitted and no fee is spent: the rejection is the contract's own answer.
# Successful calls are real transactions; their explorer links are printed.

set -uo pipefail

NETWORK="${NETWORK:-testnet}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ENV_FILE="$ROOT/.deployments/$NETWORK.env"
MERCHANT_ID="${MERCHANT_IDENTITY:-subs-demo-merchant}"
SUBSCRIBER_ID="${SUBSCRIBER_IDENTITY:-subs-demo-subscriber}"

[[ -f "$ENV_FILE" ]] || {
  echo "error: $ENV_FILE not found. Run: DEMO=1 ./scripts/deploy.sh" >&2
  exit 1
}
set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a
: "${DEMO_TOKEN:?run deploy.sh with DEMO=1 first}"

PERIOD=10000000 # 1 XLM in stroops
PASSED=0
FAILED=0
LAST_OUT=""

invoke() { # invoke <identity> <contract> <fn> [args...]
  local source="$1" id="$2"
  shift 2
  stellar contract invoke --id "$id" --source-account "$source" --network "$NETWORK" -- "$@" 2>&1
}

sub() { invoke "$1" "$SUBSCRIPTION_CONTRACT_ID" "${@:2}"; }

# Read-only call: stdout only. The CLI prints notices to stderr that would
# otherwise end up mixed into the value.
view() { # view <fn> [args...]
  stellar contract invoke --id "$SUBSCRIPTION_CONTRACT_ID" --source-account "$MERCHANT_ID" \
    --network "$NETWORK" -- "$@" 2>/dev/null
}

record() { # record <ok|fail> <message>
  if [[ "$1" == ok ]]; then
    PASSED=$((PASSED + 1))
    echo "  ok    $2"
  else
    FAILED=$((FAILED + 1))
    echo "  FAIL  $2"
  fi
}

expect_ok() { # expect_ok <label> <identity> <fn> [args...]
  local label="$1"
  shift
  if LAST_OUT="$(sub "$@")"; then
    record ok "$label"
    grep -o 'https://stellar.expert/explorer/[a-z]*/tx/[0-9a-f]*' <<<"$LAST_OUT" | sed 's/^/          /' | tail -1
  else
    record fail "$label (call failed)"
    sed 's/^/          /' <<<"$LAST_OUT" | tail -3
  fi
}

expect_reject() { # expect_reject <code> <label> <identity> <fn> [args...]
  local code="$1" label="$2"
  shift 2
  if LAST_OUT="$(sub "$@")"; then
    record fail "$label (call SUCCEEDED; the invariant is broken)"
  elif grep -q "Error(Contract, #$code)" <<<"$LAST_OUT"; then
    record ok "$label -> rejected with contract error #$code"
  else
    record fail "$label (rejected, but not with #$code)"
    sed 's/^/          /' <<<"$LAST_OUT" | tail -3
  fi
}

new_subscription() { # new_subscription <interval> <cap>  -> prints the id
  local out
  out="$(sub "$SUBSCRIBER_ID" subscribe \
    --subscriber "$DEMO_SUBSCRIBER" --merchant "$DEMO_MERCHANT" --token "$DEMO_TOKEN" \
    --amount_per_period "$PERIOD" --interval_ledgers "$1" --total_cap "$2" --plan_id 0)" || {
    echo "error: subscribe failed:" >&2
    echo "$out" >&2
    exit 1
  }
  tail -n 1 <<<"$out" | tr -d '"'
}

field() { # field <id> <json-key>
  view get_subscription --subscription_id "$1" | python3 -c \
    "import json,sys; print(json.load(sys.stdin)['$2'])"
}

charge() { sub "$MERCHANT_ID" charge --merchant "$DEMO_MERCHANT" --subscription_id "$1"; }

echo "Network:      $NETWORK"
echo "Subscription: $SUBSCRIPTION_CONTRACT_ID"
echo "Merchant:     $DEMO_MERCHANT"
echo "Subscriber:   $DEMO_SUBSCRIBER"

echo
echo "== INTERVAL, pause and REVOCATION (interval 100,000 ledgers, cap 3 charges)"
A="$(new_subscription 100000 $((3 * PERIOD)))"
echo "  subscription #$A created"
expect_ok "first charge succeeds" "$MERCHANT_ID" charge --merchant "$DEMO_MERCHANT" --subscription_id "$A"
expect_reject 11 "INTERVAL: second charge in the same interval" "$MERCHANT_ID" charge --merchant "$DEMO_MERCHANT" --subscription_id "$A"
expect_ok "subscriber pauses" "$SUBSCRIBER_ID" pause --subscriber "$DEMO_SUBSCRIBER" --subscription_id "$A"
expect_reject 15 "charge while paused" "$MERCHANT_ID" charge --merchant "$DEMO_MERCHANT" --subscription_id "$A"
expect_ok "subscriber resumes" "$SUBSCRIBER_ID" resume --subscriber "$DEMO_SUBSCRIBER" --subscription_id "$A"
expect_reject 11 "INTERVAL still holds after resume" "$MERCHANT_ID" charge --merchant "$DEMO_MERCHANT" --subscription_id "$A"
expect_reject 10 "merchant cannot cancel for the subscriber" "$MERCHANT_ID" cancel --subscriber "$DEMO_MERCHANT" --subscription_id "$A"
expect_ok "subscriber cancels" "$SUBSCRIBER_ID" cancel --subscriber "$DEMO_SUBSCRIBER" --subscription_id "$A"
expect_reject 14 "REVOCATION: charge after cancel" "$MERCHANT_ID" charge --merchant "$DEMO_MERCHANT" --subscription_id "$A"
expect_reject 14 "REVOCATION: resume after cancel" "$SUBSCRIBER_ID" resume --subscriber "$DEMO_SUBSCRIBER" --subscription_id "$A"
expect_reject 14 "REVOCATION: pause after cancel" "$SUBSCRIBER_ID" pause --subscriber "$DEMO_SUBSCRIBER" --subscription_id "$A"
expect_reject 14 "REVOCATION: cancel twice" "$SUBSCRIBER_ID" cancel --subscriber "$DEMO_SUBSCRIBER" --subscription_id "$A"
status="$(field "$A" status)"
[[ "$status" == 2 ]] && record ok "status is Cancelled (2)" || record fail "status is $status, expected 2"
charged="$(field "$A" total_charged)"
[[ "$charged" == "$PERIOD" ]] && record ok "total charged is exactly one period" || record fail "total charged is $charged"

echo
echo "== CAP (interval 1 ledger, cap 2.5 charges)"
B="$(new_subscription 1 $((PERIOD * 5 / 2)))"
echo "  subscription #$B created"
expect_ok "charge 1 of 2" "$MERCHANT_ID" charge --merchant "$DEMO_MERCHANT" --subscription_id "$B"
sleep 8
expect_ok "charge 2 of 2" "$MERCHANT_ID" charge --merchant "$DEMO_MERCHANT" --subscription_id "$B"
sleep 8
expect_reject 12 "CAP: a third charge would pass the cap" "$MERCHANT_ID" charge --merchant "$DEMO_MERCHANT" --subscription_id "$B"
left="$(view remaining_cap --subscription_id "$B" | tr -d '"')"
[[ "$left" == $((PERIOD / 2)) ]] && record ok "0.5 XLM of cap is left and can never be charged" || record fail "remaining cap is $left"
chargeable="$(view is_chargeable --subscription_id "$B")"
[[ "$chargeable" == false ]] && record ok "is_chargeable agrees: false" || record fail "is_chargeable says $chargeable"
expect_ok "subscriber cancels the leftover" "$SUBSCRIBER_ID" cancel --subscriber "$DEMO_SUBSCRIBER" --subscription_id "$B"

echo
echo "Result: $PASSED passed, $FAILED failed"
[[ "$FAILED" -eq 0 ]]
