#!/usr/bin/env bash
# scripts/check-wasm-size.sh
#
# Compare every contract's compiled wasm against the budgets in
# .github/wasm-size-budget.json and fail if any contract exceeds its budget.
#
# Usage:
#   scripts/check-wasm-size.sh              # check mode (default)
#   scripts/check-wasm-size.sh --update     # rewrite budget file with current sizes
#
# The wasm files must already exist at:
#   target/wasm32v1-none/release/<name>.wasm
#
# Run `make wasm` first if they are missing.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUDGET_FILE="$REPO_ROOT/.github/wasm-size-budget.json"
WASM_DIR="$REPO_ROOT/target/wasm32v1-none/release"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

die() { echo "ERROR: $*" >&2; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "'$1' is required but not found. Install it and retry."
}

require_cmd jq

UPDATE_MODE=0
if [[ "${1:-}" == "--update" ]]; then
  UPDATE_MODE=1
fi

# ---------------------------------------------------------------------------
# Build contract list from the budget file
# ---------------------------------------------------------------------------

mapfile -t CONTRACTS < <(jq -r '.contracts | keys[]' "$BUDGET_FILE")

if [[ ${#CONTRACTS[@]} -eq 0 ]]; then
  die "No contracts found in $BUDGET_FILE"
fi

# ---------------------------------------------------------------------------
# --update mode: rewrite budget file with current sizes
# ---------------------------------------------------------------------------

if [[ $UPDATE_MODE -eq 1 ]]; then
  echo "Updating $BUDGET_FILE with current wasm sizes..."
  UPDATED="$BUDGET_FILE"
  for contract in "${CONTRACTS[@]}"; do
    wasm_file="$WASM_DIR/${contract}.wasm"
    [[ -f "$wasm_file" ]] || die "wasm not found: $wasm_file  (run 'make wasm' first)"
    actual=$(wc -c < "$wasm_file")
    BUDGET_FILE_CONTENTS=$(jq \
      --arg contract "$contract" \
      --argjson size "$actual" \
      '.contracts[$contract].budget_bytes = $size' \
      "$BUDGET_FILE")
    echo "$BUDGET_FILE_CONTENTS" > "$BUDGET_FILE"
    echo "  $contract: $actual bytes"
  done
  echo "Done. Commit the updated $BUDGET_FILE with a note explaining the size change."
  exit 0
fi

# ---------------------------------------------------------------------------
# Check mode: compare actual sizes against budgets
# ---------------------------------------------------------------------------

PASS=1
printf "\n%-20s %12s %12s %10s\n" "CONTRACT" "ACTUAL (B)" "BUDGET (B)" "STATUS"
printf "%s\n" "------------------------------------------------------------"

for contract in "${CONTRACTS[@]}"; do
  wasm_file="$WASM_DIR/${contract}.wasm"
  [[ -f "$wasm_file" ]] || die "wasm not found: $wasm_file  (run 'make wasm' first)"

  actual=$(wc -c < "$wasm_file")
  budget=$(jq -r --arg c "$contract" '.contracts[$c].budget_bytes' "$BUDGET_FILE")

  if [[ "$budget" == "null" ]]; then
    die "No budget entry for contract '$contract' in $BUDGET_FILE"
  fi

  if (( actual <= budget )); then
    status="OK"
    over=""
  else
    status="OVER BUDGET"
    over=" (+$(( actual - budget )) B)"
    PASS=0
  fi

  printf "%-20s %12d %12d %10s%s\n" "$contract" "$actual" "$budget" "$status" "$over"
done

printf "%s\n\n" "------------------------------------------------------------"

if [[ $PASS -eq 0 ]]; then
  echo "FAIL: One or more contracts exceed their wasm size budget."
  echo "  • If the growth is intentional, run:  scripts/check-wasm-size.sh --update"
  echo "    and commit the updated .github/wasm-size-budget.json with a PR comment"
  echo "    explaining why the extra bytes are worth it."
  echo "  • Otherwise, reduce the contract's footprint before merging."
  exit 1
else
  echo "All contracts are within their wasm size budget."
  exit 0
fi
