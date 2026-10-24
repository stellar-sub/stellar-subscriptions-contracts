# Merchant Integration Guide — Stellar CLI

This guide walks a merchant through the full lifecycle of a recurring
subscription billing plan using only the Stellar CLI. No SDK or backend
service is required. Every command has been verified against the live Testnet
deployment.

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Contract addresses](#2-contract-addresses)
3. [Creating a billing plan](#3-creating-a-billing-plan)
4. [Listing your plans](#4-listing-your-plans)
5. [Checking whether a subscription is chargeable](#5-checking-whether-a-subscription-is-chargeable)
6. [Charging a subscriber](#6-charging-a-subscriber)
7. [Listing your subscribers](#7-listing-your-subscribers)
8. [Deactivating a plan](#8-deactivating-a-plan)
9. [Common errors](#9-common-errors)

---

## 1. Prerequisites

### Install the Stellar CLI

```bash
cargo install --locked stellar-cli --features opt
```

Verify the installation:

```bash
stellar --version
# stellar 21.x.x
```

### Create and fund a merchant identity

Generate a new keypair and name it `merchant`:

```bash
stellar keys generate --global merchant --network testnet
```

Fund the account on Testnet via Friendbot:

```bash
stellar keys fund --network testnet merchant
```

Confirm the account is live:

```bash
stellar account show --network testnet $(stellar keys address merchant)
```

### Configure the CLI network defaults

Set Testnet as the default so you do not have to pass `--network testnet`
to every command:

```bash
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

stellar network use testnet
```

> **Tip:** All commands below omit `--network testnet` for brevity. If you
> have not set a default, append `--network testnet` to every invocation.

---

## 2. Contract addresses

These are the live Testnet addresses (protocol 28). Use them verbatim in
every command.

| Contract | Address |
|---|---|
| Subscription | `CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM` |
| Plan | `CDKFRKPA2EIYMW2GX2FY4ZFQ2VAFS5UZD5BHAZLCIGRA7N24N2DFAYN3` |
| Registry | `CB7HBXFNBLUSBY3SMIYUPVTE3JZTISYRMZB7XXPUC3ZPLIDNPTKUX645` |
| XLM (SAC) | `CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC` |

Shell variables used throughout this guide:

```bash
PLAN_CONTRACT=CDKFRKPA2EIYMW2GX2FY4ZFQ2VAFS5UZD5BHAZLCIGRA7N24N2DFAYN3
SUB_CONTRACT=CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM
REG_CONTRACT=CB7HBXFNBLUSBY3SMIYUPVTE3JZTISYRMZB7XXPUC3ZPLIDNPTKUX645
XLM_TOKEN=CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC
MERCHANT=$(stellar keys address merchant)
```

---

## 3. Creating a billing plan

`create_plan` registers a reusable set of billing terms — token, amount,
interval, and a suggested total cap — that subscribers can use when
subscribing. The function returns the new plan ID (a `u64`).

### Argument reference

| Argument | Type | Meaning |
|---|---|---|
| `merchant` | `Address` | Your identity. Must sign the transaction. |
| `name` | `String` | Plan label shown to subscribers. 1–64 bytes. |
| `token` | `Address` | The token to charge in (use the XLM SAC for XLM). |
| `amount_per_period` | `i128` | Amount charged each billing cycle, in the token's smallest unit (stroops for XLM; 1 XLM = 10 000 000 stroops). |
| `interval_ledgers` | `u32` | Minimum ledgers between consecutive charges. Testnet produces ~1 ledger per 5 s; 17 280 ledgers ≈ 1 day. |
| `default_cap` | `i128` | Suggested total cap offered to subscribers at subscribe time. Must be ≥ `amount_per_period`. Subscribers may choose their own cap. |

### Example: 10 XLM/month plan

Testnet produces roughly 17 280 ledgers per day, so 30 days ≈ 518 400
ledgers. 10 XLM = 100 000 000 stroops.

```bash
stellar contract invoke \
  --id $PLAN_CONTRACT \
  --source merchant \
  --send yes \
  -- \
  create_plan \
  --merchant $MERCHANT \
  --name "Pro Monthly" \
  --token $XLM_TOKEN \
  --amount_per_period 100000000 \
  --interval_ledgers 518400 \
  --default_cap 1200000000
```

The CLI prints the returned plan ID, for example:

```
1
```

Save it for later steps:

```bash
PLAN_ID=1
```

> **Note:** `default_cap 1200000000` is 120 XLM — enough for 12 months.
> Subscribers are free to set a lower or higher cap when they subscribe.

### Example: 5 XLM/week plan

```bash
stellar contract invoke \
  --id $PLAN_CONTRACT \
  --source merchant \
  --send yes \
  -- \
  create_plan \
  --merchant $MERCHANT \
  --name "Basic Weekly" \
  --token $XLM_TOKEN \
  --amount_per_period 50000000 \
  --interval_ledgers 120960 \
  --default_cap 260000000
```

---

## 4. Listing your plans

`get_plans_by_merchant` returns all plans (active and inactive) you have
created, oldest first. It is a read-only view — no signature required.

```bash
stellar contract invoke \
  --id $PLAN_CONTRACT \
  --source merchant \
  -- \
  get_plans_by_merchant \
  --merchant $MERCHANT
```

Example output (pretty-printed):

```json
[
  {
    "id": 1,
    "merchant": "GDXYZ...",
    "name": "Pro Monthly",
    "token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
    "amount_per_period": 100000000,
    "interval_ledgers": 518400,
    "default_cap": 1200000000,
    "active": true,
    "created_at": 1234567
  }
]
```

To list all active plans across all merchants:

```bash
stellar contract invoke \
  --id $PLAN_CONTRACT \
  --source merchant \
  -- \
  get_active_plans
```

---

## 5. Checking whether a subscription is chargeable

Before spending a transaction fee on a charge attempt, call `is_chargeable`
to confirm the subscription is in a billable state. It returns `true` only
when all of these hold:

- status is `Active`
- the current ledger is at or past `next_charge_ledger`
- charging would not exceed `total_cap`

```bash
stellar contract invoke \
  --id $SUB_CONTRACT \
  --source merchant \
  -- \
  is_chargeable \
  --subscription_id 42
```

Returns `true` or `false`.

### Retrieve the full subscription record

For more detail — including `next_charge_ledger`, `total_charged`, `status`,
and `total_cap` — use `get_subscription`:

```bash
stellar contract invoke \
  --id $SUB_CONTRACT \
  --source merchant \
  -- \
  get_subscription \
  --subscription_id 42
```

Example output:

```json
{
  "id": 42,
  "subscriber": "GABC...",
  "merchant": "GDXYZ...",
  "token": "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
  "amount_per_period": 100000000,
  "interval_ledgers": 518400,
  "total_cap": 1200000000,
  "total_charged": 100000000,
  "last_charge_ledger": 1300000,
  "next_charge_ledger": 1818400,
  "status": "Active",
  "plan_id": 1
}
```

`status` values: `Active` (0), `Paused` (1), `Cancelled` (2), `Exhausted` (3).

---

## 6. Charging a subscriber

`charge` pulls `amount_per_period` from the subscriber's wallet to yours.
Only the merchant named in the subscription may call it.

```bash
stellar contract invoke \
  --id $SUB_CONTRACT \
  --source merchant \
  --send yes \
  -- \
  charge \
  --merchant $MERCHANT \
  --subscription_id 42
```

On success the CLI prints nothing (the function returns `void`). The contract
emits a `charged` event you can observe in the Stellar Laboratory or your own
event stream.

### Automating billing

For production, run `is_chargeable` and then `charge` for every subscription
returned by `get_by_merchant`. A minimal shell loop:

```bash
# Fetch all subscription IDs for your merchant
IDS=$(stellar contract invoke \
  --id $SUB_CONTRACT \
  --source merchant \
  -- \
  get_by_merchant \
  --merchant $MERCHANT)

# IDS is a JSON array: [42, 73, 101, ...]
for ID in $(echo $IDS | jq '.[]'); do
  CHARGEABLE=$(stellar contract invoke \
    --id $SUB_CONTRACT \
    --source merchant \
    -- \
    is_chargeable \
    --subscription_id $ID)

  if [ "$CHARGEABLE" = "true" ]; then
    echo "Charging subscription $ID..."
    stellar contract invoke \
      --id $SUB_CONTRACT \
      --source merchant \
      --send yes \
      -- \
      charge \
      --merchant $MERCHANT \
      --subscription_id $ID
  else
    echo "Subscription $ID not chargeable yet, skipping."
  fi
done
```

> **Tip:** Schedule this script with cron or a CI job. Charging late is safe —
> the contract schedules the *next* charge from the ledger the charge
> actually lands on, so you cannot cause double charges by retrying after a
> missed interval.

---

## 7. Listing your subscribers

### Via the Subscription contract

`get_by_merchant` returns the IDs of every subscription (including
cancelled and exhausted) ever created for your merchant address:

```bash
stellar contract invoke \
  --id $SUB_CONTRACT \
  --source merchant \
  -- \
  get_by_merchant \
  --merchant $MERCHANT
```

Returns a JSON array of subscription IDs, e.g. `[1, 2, 42, 73]`. Use
`get_subscription` on each ID for full details (see §5).

### Via the Registry

The Registry provides aggregate stats and filtered lists without iterating
over the subscription contract yourself.

**Global stats:**

```bash
stellar contract invoke \
  --id $REG_CONTRACT \
  --source merchant \
  -- \
  get_stats
```

Example output:

```json
{
  "total_plans": 3,
  "total_subscriptions": 127,
  "active_subscriptions": 98,
  "total_charged_volume": 9800000000
}
```

`total_charged_volume` is in raw token units (stroops) across all tokens — it
is an activity signal, not a currency total.

**All active subscription IDs:**

```bash
stellar contract invoke \
  --id $REG_CONTRACT \
  --source merchant \
  -- \
  get_active_subscriptions
```

Returns `[42, 73, ...]` — the IDs of every subscription last reported as
`Active`.

**All subscription IDs ever registered:**

```bash
stellar contract invoke \
  --id $REG_CONTRACT \
  --source merchant \
  -- \
  get_all_subscriptions
```

**Summary record for a single subscription:**

```bash
stellar contract invoke \
  --id $REG_CONTRACT \
  --source merchant \
  -- \
  get_subscription_record \
  --subscription_id 42
```

Example output:

```json
{
  "id": 42,
  "subscriber": "GABC...",
  "merchant": "GDXYZ...",
  "total_charged": 100000000,
  "status": 0
}
```

`status` 0 = Active, 1 = Paused, 2 = Cancelled, 3 = Exhausted.

> **Note:** The Registry only knows subscriptions created after the registry
> was linked. Subscriptions created before that date appear only in the
> Subscription contract's own storage.

---

## 8. Deactivating a plan

`deactivate_plan` marks a plan as inactive. It prevents new subscribers from
using the plan, but **has no effect on existing subscriptions** — subscribers
who already agreed to the plan's terms continue to be chargeable under their
original authorization.

```bash
stellar contract invoke \
  --id $PLAN_CONTRACT \
  --source merchant \
  --send yes \
  -- \
  deactivate_plan \
  --merchant $MERCHANT \
  --plan_id $PLAN_ID
```

Deactivation is **irreversible**. To stop billing existing subscribers you
must simply stop calling `charge` — the contract has no way to force-cancel
subscriptions on a merchant's behalf (only the subscriber can cancel).

Verify the plan is now inactive:

```bash
stellar contract invoke \
  --id $PLAN_CONTRACT \
  --source merchant \
  -- \
  get_plan \
  --plan_id $PLAN_ID
```

The returned record will show `"active": false`.

---

## 9. Common errors

The contract rejects calls by returning a numeric error code. The Stellar CLI
surfaces it as a transaction failure. Below are the errors you are most likely
to encounter as a merchant, what they mean, and how to resolve them.

Full error tables are in [docs/subscription.md](subscription.md) and
[docs/plan.md](plan.md).

---

### `IntervalNotElapsed` — code 11

**When:** `charge` is called before `next_charge_ledger` has been reached.

**Why it happens:**
- You are charging too early — the billing interval has not elapsed since the
  last successful charge.
- A previous billing run already charged this subscription in the current
  window.

**How to fix:** Call `is_chargeable` before `charge`. If it returns `false`,
read `next_charge_ledger` from `get_subscription` and wait until the network
reaches that ledger. At ~5 s per ledger on Testnet, you can estimate the
wall-clock time remaining.

---

### `CapExceeded` — code 12

**When:** The charge would push `total_charged` above the subscriber's
`total_cap`.

**Why it happens:** The subscriber set a cap smaller than the remaining
balance you expect, or a previous billing run consumed the last period
before the cap.

**How to fix:** Check `remaining_cap` (= `total_cap - total_charged`) via
`get_subscription`. If it is less than `amount_per_period`, the subscription
is at capacity. `is_chargeable` will also return `false` in this state.
The subscription stays `Active`; the subscriber can re-authorize a higher cap
by creating a new subscription or (if a `refresh_allowance` flow is added)
updating their allowance. You cannot raise the cap yourself.

---

### `SubscriptionCancelled` — code 14

**When:** The subscriber has called `cancel`. The subscription is permanently
closed.

**Why it happens:** The subscriber explicitly revoked consent. This is a
deliberate, final act on their part.

**How to fix:** Nothing — a cancelled subscription can never be charged again.
Remove it from your billing list. If the subscriber wants to resume, they must
create a new subscription.

---

### `SubscriptionPaused` — code 15

**When:** The subscription's status is `Paused`.

**Why it happens:** The subscriber temporarily paused billing using `pause`.
Paused periods are never billed retroactively.

**How to fix:** Wait for the subscriber to call `resume`. `is_chargeable`
returns `false` while paused. You can monitor the `resumed` event or poll
`get_subscription` to detect when the subscription returns to `Active`.

---

### `SubscriptionExhausted` — code 16

**When:** A prior `charge` brought `total_charged` exactly to `total_cap`.
The subscription is now `Exhausted` — a terminal state.

**Why it happens:** This is normal end-of-life for a subscription that hit
its authorized ceiling. It is not an error in the fault sense.

**How to fix:** Remove the subscription from your billing list. Encourage the
subscriber to create a new subscription with a higher cap if they wish to
continue.

---

### `NotMerchant` — code 9

**When:** The `merchant` argument to `charge` does not match the address
recorded in the subscription.

**Why it happens:** Using the wrong identity, wrong `--source`, or the wrong
subscription ID.

**How to fix:** Confirm `$(stellar keys address merchant)` matches the
`merchant` field in `get_subscription`. Use `get_by_merchant` to retrieve only
subscriptions that belong to your address.

---

### `TransferFailed` — code 19

**When:** The token contract refused the `transfer_from`. No state changed —
the interval and cap are unchanged, so you may retry.

**Why it happens — most common causes:**
- The subscriber's token allowance for the subscription contract has expired.
  Soroban allowances have a maximum lifetime (~120 days on Mainnet); a
  long-running subscription needs `refresh_allowance` called by the subscriber
  before the expiry ledger.
- The subscriber's token balance is insufficient.

**How to fix:**
- Ask the subscriber to call `refresh_allowance` on the subscription contract
  to renew their allowance.
- If balance is the issue, there is nothing to do until the subscriber funds
  their account.

Because no state changed, retrying `charge` after the subscriber refreshes
will succeed (subject to the interval and cap checks).

---

### Quick-reference table

| Code | Name | `charge` fails because… | Retryable? |
|---|---|---|---|
| 9 | `NotMerchant` | Wrong identity or wrong subscription | Fix source / ID |
| 11 | `IntervalNotElapsed` | Too early | Wait |
| 12 | `CapExceeded` | Subscriber's cap exhausted | No (subscriber must act) |
| 14 | `SubscriptionCancelled` | Subscriber cancelled | No (final) |
| 15 | `SubscriptionPaused` | Subscriber paused | Wait for resume |
| 16 | `SubscriptionExhausted` | Cap reached exactly on last charge | No (final) |
| 19 | `TransferFailed` | Token allowance expired or balance low | After subscriber refreshes |

---

*Commands in this guide were verified against the live Testnet deployment
(protocol 28). Contract addresses are as listed in
[DEPLOYMENTS.md](../DEPLOYMENTS.md).*
