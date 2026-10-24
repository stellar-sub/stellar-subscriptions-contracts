# Events reference

All state changes in the Stellar Subscriptions contracts are recorded as
Soroban contract events. This document lists every event, its topics (in
order), its data fields and types, when it is emitted, and one decoded
example taken from Testnet.

> **Keeping this in sync.** Every event struct is defined in source code
> (listed in the *Source* column). When adding a new event or changing a
> field, update this document in the same PR. The PR checklist in
> [CONTRIBUTING.md](../CONTRIBUTING.md) includes this step.

---

## Subscription contract

Source: `contracts/subscription/src/events.rs`

### `subscribed`

Emitted when a subscriber creates a new subscription.

| Position | Kind | Type | Value |
|---|---|---|---|
| topic 0 | event name | `Symbol` | `"subscribed"` |
| topic 1 | subscription_id | `u64` | e.g. `1` |
| topic 2 | subscriber | `Address` | subscriber's Stellar address |
| topic 3 | merchant | `Address` | merchant's Stellar address |
| data 0 | token | `Address` | token contract address |
| data 1 | amount_per_period | `i128` | e.g. `10000000` |
| data 2 | interval_ledgers | `u32` | e.g. `720` |
| data 3 | total_cap | `i128` | e.g. `120000000` |
| data 4 | plan_id | `u64` | `0` for ad-hoc, non-zero for plan-based |

**When emitted:** at the end of a successful `subscribe()` call, after the
token allowance is approved and the registry is updated.

**Decoded example (Testnet, subscription 1):**

```json
{
  "type": "contract",
  "ledger": 4692992,
  "contractId": "CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM",
  "topics": [
    "subscribed",
    1,
    "GBKAVFZ5GCZMMIV3ZKZL5USKFYT5DBBZTY4P5V5J75ANHWTX7GASETK4",
    "GCLAL5UFF47MT7JMJHSS4BWXMMKJSTGYE62HJ5X77N45UKLF7BIDKKTU"
  ],
  "data": [
    "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
    10000000,
    720,
    120000000,
    1
  ]
}
```

---

### `charged`

Emitted when a merchant successfully pulls one period's payment.

| Position | Kind | Type | Value |
|---|---|---|---|
| topic 0 | event name | `Symbol` | `"charged"` |
| topic 1 | subscription_id | `u64` | e.g. `1` |
| topic 2 | merchant | `Address` | merchant's Stellar address |
| data 0 | amount | `i128` | amount transferred this charge |
| data 1 | total_charged | `i128` | cumulative total charged so far |
| data 2 | next_charge_ledger | `u32` | earliest ledger the next charge may land |
| data 3 | exhausted | `bool` | `true` if this charge reached `total_cap` |

**When emitted:** after state is written and the token transfer succeeds.
A failed charge emits nothing.

**Decoded example (Testnet, charge on subscription 1):**

```json
{
  "type": "contract",
  "ledger": 4692994,
  "contractId": "CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM",
  "topics": [
    "charged",
    1,
    "GCLAL5UFF47MT7JMJHSS4BWXMMKJSTGYE62HJ5X77N45UKLF7BIDKKTU"
  ],
  "data": [
    10000000,
    10000000,
    4693714,
    false
  ]
}
```

---

### `cancelled`

Emitted when a subscriber cancels. Cancellation is final; no further events
will be emitted for this subscription.

| Position | Kind | Type | Value |
|---|---|---|---|
| topic 0 | event name | `Symbol` | `"cancelled"` |
| topic 1 | subscription_id | `u64` | e.g. `1` |
| topic 2 | subscriber | `Address` | subscriber's Stellar address |
| data 0 | total_charged | `i128` | total charged before cancellation |
| data 1 | unused_cap | `i128` | cap that was authorized but never charged |
| data 2 | allowance_updated | `bool` | whether the token allowance was cleaned up |
| data 3 | registry_updated | `bool` | whether the registry recorded the cancellation |

**When emitted:** at the end of `cancel()`, after state is written.
`allowance_updated` and `registry_updated` may be `false` if the
corresponding follow-up call failed; the cancellation itself is always
effective.

**Decoded example (Testnet):**

```json
{
  "type": "contract",
  "ledger": 4693010,
  "contractId": "CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM",
  "topics": [
    "cancelled",
    4,
    "GBKAVFZ5GCZMMIV3ZKZL5USKFYT5DBBZTY4P5V5J75ANHWTX7GASETK4"
  ],
  "data": [
    10000000,
    110000000,
    true,
    true
  ]
}
```

---

### `paused`

Emitted when a subscriber pauses charging.

| Position | Kind | Type | Value |
|---|---|---|---|
| topic 0 | event name | `Symbol` | `"paused"` |
| topic 1 | subscription_id | `u64` | e.g. `1` |
| topic 2 | subscriber | `Address` | subscriber's Stellar address |
| data 0 | ledger | `u32` | ledger sequence at which the pause took effect |

**When emitted:** at the end of a successful `pause()` call.

**Decoded example (Testnet):**

```json
{
  "type": "contract",
  "ledger": 4693001,
  "contractId": "CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM",
  "topics": [
    "paused",
    4,
    "GBKAVFZ5GCZMMIV3ZKZL5USKFYT5DBBZTY4P5V5J75ANHWTX7GASETK4"
  ],
  "data": [4693001]
}
```

---

### `resumed`

Emitted when a subscriber resumes charging after a pause.

| Position | Kind | Type | Value |
|---|---|---|---|
| topic 0 | event name | `Symbol` | `"resumed"` |
| topic 1 | subscription_id | `u64` | e.g. `1` |
| topic 2 | subscriber | `Address` | subscriber's Stellar address |
| data 0 | next_charge_ledger | `u32` | earliest ledger the next charge may land |

**When emitted:** at the end of a successful `resume()` call.

**Decoded example (Testnet):**

```json
{
  "type": "contract",
  "ledger": 4693005,
  "contractId": "CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM",
  "topics": [
    "resumed",
    4,
    "GBKAVFZ5GCZMMIV3ZKZL5USKFYT5DBBZTY4P5V5J75ANHWTX7GASETK4"
  ],
  "data": [4693001]
}
```

---

### `allowance_refreshed`

Emitted when a subscriber re-approves the contract's spending allowance with
a fresh expiry.

| Position | Kind | Type | Value |
|---|---|---|---|
| topic 0 | event name | `Symbol` | `"allowance_refreshed"` |
| topic 1 | subscriber | `Address` | subscriber's Stellar address |
| topic 2 | token | `Address` | token contract address |
| data 0 | amount | `i128` | new allowance amount approved |

**Note:** unlike the other subscription events, this event is keyed by
`subscriber` + `token`, not by `subscription_id`, because the allowance is
shared across all of a subscriber's active subscriptions in that token.

**When emitted:** at the end of a successful `refresh_allowance()` call.

**Decoded example:**

```json
{
  "type": "contract",
  "ledger": 4694200,
  "contractId": "CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM",
  "topics": [
    "allowance_refreshed",
    "GBKAVFZ5GCZMMIV3ZKZL5USKFYT5DBBZTY4P5V5J75ANHWTX7GASETK4",
    "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC"
  ],
  "data": [110000000]
}
```

---

## Plan contract

Source: `contracts/plan/src/lib.rs`

### `plan_created`

Emitted when a merchant publishes a new billing plan.

| Position | Kind | Type | Value |
|---|---|---|---|
| topic 0 | event name | `Symbol` | `"plan_created"` |
| topic 1 | plan_id | `u64` | e.g. `1` |
| topic 2 | merchant | `Address` | merchant's Stellar address |
| data 0 | name | `String` | plan display name (1–64 bytes) |
| data 1 | token | `Address` | token contract address |
| data 2 | amount_per_period | `i128` | e.g. `10000000` |
| data 3 | interval_ledgers | `u32` | e.g. `720` |
| data 4 | default_cap | `i128` | suggested cap (>= amount_per_period) |

**When emitted:** at the end of a successful `create_plan()` call, after the
plan is registered with the registry.

**Decoded example (Testnet, Plan 1 "Demo hourly"):**

```json
{
  "type": "contract",
  "ledger": 4692990,
  "contractId": "CDKFRKPA2EIYMW2GX2FY4ZFQ2VAFS5UZD5BHAZLCIGRA7N24N2DFAYN3",
  "topics": [
    "plan_created",
    1,
    "GCLAL5UFF47MT7JMJHSS4BWXMMKJSTGYE62HJ5X77N45UKLF7BIDKKTU"
  ],
  "data": [
    "Demo hourly",
    "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC",
    10000000,
    720,
    120000000
  ]
}
```

---

### `plan_deactivated`

Emitted when a merchant deactivates a plan. Deactivation is irreversible.

| Position | Kind | Type | Value |
|---|---|---|---|
| topic 0 | event name | `Symbol` | `"plan_deactivated"` |
| topic 1 | plan_id | `u64` | e.g. `1` |
| topic 2 | merchant | `Address` | merchant's Stellar address |

No data fields.

**When emitted:** at the end of a successful `deactivate_plan()` call.

**Decoded example:**

```json
{
  "type": "contract",
  "ledger": 4695000,
  "contractId": "CDKFRKPA2EIYMW2GX2FY4ZFQ2VAFS5UZD5BHAZLCIGRA7N24N2DFAYN3",
  "topics": [
    "plan_deactivated",
    1,
    "GCLAL5UFF47MT7JMJHSS4BWXMMKJSTGYE62HJ5X77N45UKLF7BIDKKTU"
  ],
  "data": []
}
```

---

## Registry contract

Source: `contracts/registry/src/lib.rs`

The registry contract emits **no events**. It is a read-optimised index
updated by calls from the subscription and plan contracts. All meaningful
state changes are recorded by those contracts' events; the registry's own
state can be reconstructed by replaying them.

---

## Querying events

To fetch events from the Testnet deployment using the Stellar CLI:

```bash
stellar events \
  --network testnet \
  --contract-id CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM \
  --start-ledger 4692990
```

To filter to a specific event type (e.g., only `charged` events):

```bash
stellar events \
  --network testnet \
  --contract-id CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM \
  --start-ledger 4692990 \
  --topic1 charged
```

See [DEPLOYMENTS.md](../DEPLOYMENTS.md) for all contract ids.
