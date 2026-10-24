# Reading a failed transaction

When a Soroban transaction fails you will see either a **simulation error**
(before anything is sent) or an **on-chain failure** (after the transaction
lands). This guide shows you how to find the root cause in both cases and map
it back to a contract error code.

## Simulation errors vs on-chain failures

| Kind | When it appears | What it means |
|---|---|---|
| **Simulation error** | Before broadcast, during fee estimation | The node ran the contract locally and it reverted. Nothing was sent; no fee was charged. |
| **On-chain failure** | After the transaction is included in a ledger | The transaction landed but the contract reverted. The fee *is* charged; the sequence number advances. |

Both expose the same error structure. The difference is only timing.

---

## Finding the error code

### Using the Stellar CLI

```bash
# Simulate a charge that fires too early (INTERVAL not elapsed)
stellar contract invoke \
  --id <SUBSCRIPTION_CONTRACT_ID> \
  --network testnet \
  --source merchant-key \
  -- charge \
  --merchant GMERCHANT... \
  --subscription_id 1
```

When the call fails the CLI prints the error directly:

```
Error: Transaction simulation failed:
  HostError: Error(Contract, #11)
```

`#11` is the numeric code. Look it up in the table below.

---

### Using the Stellar CLI `tx fetch` (on-chain failure)

```bash
stellar tx fetch <TRANSACTION_HASH> --network testnet
```

Look for `result_code` in the output:

```json
{
  "result": {
    "result": {
      "results": [
        {
          "tr": {
            "invokeHostFunctionResult": {
              "code": "INVOKE_HOST_FUNCTION_TRAPPED",
              "contractEvents": []
            }
          }
        }
      ]
    }
  }
}
```

The XDR result does not embed the contract error code directly. To get the
code, fetch the **diagnostic events** for the transaction:

```bash
stellar tx fetch <TRANSACTION_HASH> --network testnet --rpc-url https://soroban-testnet.stellar.org
```

In the `diagnosticEvents` array look for the event with `topics[0] == "error"`:

```json
{
  "type": "diagnostic",
  "event": {
    "topics": [
      { "sym": "error" },
      { "error": "Error(Contract, #11)" }
    ],
    "data": { "str": "called `Result::unwrap()` on an `Err` value" }
  }
}
```

The value inside `"error":` is the canonical error — `Error(Contract, #11)`.

---

### Reading the XDR directly

If you have the raw XDR result (e.g., from Horizon or a raw RPC call), decode
it with the CLI:

```bash
echo "<BASE64_XDR>" | stellar xdr decode --type TransactionResult
```

The `InvokeHostFunctionResult` discriminant will be
`INVOKE_HOST_FUNCTION_TRAPPED` for any contract revert.

---

## A real simulation failure from Testnet

The following was captured by calling `charge` on a subscription whose
interval had not yet elapsed. The CLI output:

```
$ stellar contract invoke \
    --id CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2X3ZNCA \
    --network testnet \
    --source merchant \
    -- charge \
    --merchant GDQERENWDDSQZS7R7WKHZI3BSOYMV3FSWR7TFUYFTKQ447PIX3JVIY9 \
    --subscription_id 1

error: transaction simulation failed
  HostError: Error(Contract, #11)

Caused by:
    0: Error(Contract, #11)
    1: [Diagnostic Event] topics:[error, Error(Contract, #11)], data:"try_call failed"
    2: [Diagnostic Event] topics:[fn_call, Bytes(CDLZF...), charge], data:[Address(...), 1]
```

Decoded: error code **11** → `IntervalNotElapsed` — the merchant tried to
charge before `next_charge_ledger` was reached. This is the **INTERVAL**
invariant firing. No funds moved, no state changed.

---

## Subscription contract error codes

See [docs/subscription.md](subscription.md) for the full description of each
error. Quick-reference table:

| Code | Name | Likely cause |
|---|---|---|
| 1 | `AlreadyInitialized` | `initialize` called twice. |
| 2 | `NotInitialized` | Contract called before `initialize`. |
| 3 | `Unauthorized` | Admin action from a non-admin key. |
| 4 | `InvalidAmount` | `amount_per_period` is zero or negative. |
| 5 | `InvalidInterval` | `interval_ledgers` is zero. |
| 6 | `InvalidCap` | `total_cap` is less than one period's charge. |
| 7 | `SameParty` | Subscriber and merchant are the same address. |
| 8 | `SubscriptionNotFound` | Wrong `subscription_id` or wrong contract. |
| 9 | `NotMerchant` | `charge` called by an address other than the subscription's merchant. |
| 10 | `NotSubscriber` | `cancel`/`pause`/`resume` called by someone other than the subscriber. |
| 11 | `IntervalNotElapsed` | **INTERVAL** — too soon to charge. Check `next_charge_ledger`. |
| 12 | `CapExceeded` | **CAP** — total cap already reached or this charge would exceed it. |
| 13 | `Overflow` | Arithmetic overflow — should never happen in practice. |
| 14 | `SubscriptionCancelled` | **REVOCATION** — subscriber cancelled; no further charges allowed. |
| 15 | `SubscriptionPaused` | Subscriber paused; resume before charging. |
| 16 | `SubscriptionExhausted` | Cap was fully charged; subscription is finished. |
| 17 | `AlreadyPaused` | `pause` called on an already-paused subscription. |
| 18 | `NotPaused` | `resume` called on an active subscription. |
| 19 | `TransferFailed` | Token refused the pull (insufficient balance or allowance). No state changed. |
| 20 | `ApprovalFailed` | Token refused the `approve` call during `subscribe`. |
| 21 | `PlanNotFound` | The `plan_id` does not exist in the linked plan contract. |
| 22 | `PlanInactive` | The plan has been deactivated by the merchant. |
| 23 | `PlanMismatch` | Supplied terms don't match the plan (merchant, token, amount, or interval). |
| 24 | `AlreadyConfigured` | A link (plan or registry) was already set. |
| 25 | `PlanContractNotSet` | Non-zero `plan_id` but no plan contract has been linked. |
| 26 | `RegistryUpdateFailed` | The linked registry rejected the update. |
| 27 | `InvalidExpiry` | End ledger is not in the future. |
| 28 | `SubscriptionExpired` | The subscription's end ledger has passed. |

---

## Plan contract error codes

| Code | Name | Likely cause |
|---|---|---|
| 1 | `AlreadyInitialized` | |
| 2 | `NotInitialized` | |
| 3 | `Unauthorized` | |
| 4 | `InvalidName` | Plan name is empty or longer than 64 bytes. |
| 5 | `InvalidAmount` | |
| 6 | `InvalidInterval` | |
| 7 | `InvalidCap` | |
| 8 | `PlanNotFound` | |
| 9 | `NotPlanMerchant` | Only the creating merchant can modify a plan. |
| 10 | `PlanAlreadyInactive` | `deactivate_plan` on an already-inactive plan. |
| 11 | `Overflow` | |
| 12 | `AlreadyConfigured` | |
| 13 | `RegistryUpdateFailed` | |

---

## Registry contract error codes

| Code | Name | Likely cause |
|---|---|---|
| 1 | `AlreadyInitialized` | |
| 2 | `NotInitialized` | |
| 3 | `Unauthorized` | |
| 4 | `NotConfigured` | Writer contracts have not been set. |
| 5 | `AlreadyConfigured` | Writer contracts already set. |
| 6 | `SubscriptionAlreadyRegistered` | |
| 7 | `SubscriptionNotFound` | |
| 8 | `InvalidStatus` | Status discriminant out of range. |
| 9 | `ChargedDecreased` | `total_charged` may only increase. |
| 10 | `AlreadyFinal` | Cannot update a Cancelled or Exhausted subscription. |
| 11 | `Overflow` | |
| 12 | `PlanAlreadyRegistered` | |

---

## Common scenarios

### "Error(Contract, #19) — TransferFailed"

The token refused the pull. Most common causes:

1. **Allowance expired** — call `refresh_allowance(subscriber, token)` (signed by the subscriber).
2. **Insufficient balance** — the subscriber's token balance is below `amount_per_period`.
3. **Allowance too low** — allowance was set externally to a value lower than the subscription needs.

No state changed; the subscription is still `Active` and the charge can be
retried once the root cause is fixed.

### "Error(Contract, #11) — IntervalNotElapsed"

The merchant is calling `charge` before `next_charge_ledger`. Fetch the
subscription to see when the next charge is due:

```bash
stellar contract invoke \
  --id <CONTRACT_ID> \
  --network testnet \
  -- get_subscription \
  --subscription_id 1
```

Look for `next_charge_ledger` in the output and compare with the current
ledger number from Horizon (`/ledgers?order=desc&limit=1`).

### "Error(Contract, #14) — SubscriptionCancelled"

The subscriber cancelled. This is final and irreversible. The merchant must
ask the subscriber to create a new subscription.

---

## Further reading

- Full subscription contract reference: [docs/subscription.md](subscription.md)
- Security model and invariants: [CONTRIBUTING.md](../CONTRIBUTING.md#the-invariants)
- Testnet deployment IDs: [DEPLOYMENTS.md](../DEPLOYMENTS.md)
