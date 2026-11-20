# Plan contract

`contracts/plan` — reusable billing terms published by merchants.

A plan is a template: token, amount per period, interval, and a suggested
cap. Subscribers subscribe to a plan through the subscription contract, which
checks that the subscription's terms match the plan exactly. The subscriber
still picks their own cap.

A plan **never moves funds** and has **no authority over subscriptions**.
Deactivating one only stops new subscriptions to it.

## Data

```rust
pub struct Plan {
    pub id: u64,               // starts at 1; 0 means "no plan" to subscriptions
    pub merchant: Address,
    pub name: String,          // 1..=64 bytes
    pub token: Address,
    pub amount_per_period: i128,
    pub interval_ledgers: u32,
    pub default_cap: i128,     // suggestion, >= amount_per_period
    pub active: bool,
    pub created_at: u32,       // ledger sequence
    pub description: Option<String>, // None or 1..=MAX_DESCRIPTION_LEN bytes
    pub subscriber_count: u64, // incremented on each subscribe naming this plan
}
```

The subscription contract reads `Plan` across contracts and decodes it by
field name, so **field names must not change**.

`description` is optional free-form text (e.g. a human-readable summary of the
plan). If provided it must be non-empty and at most `MAX_DESCRIPTION_LEN`
(256) bytes; longer descriptions are rejected with `InvalidDescription`.
`subscriber_count` starts at 0 and is incremented by the subscription contract
on every successful `subscribe` that names this plan. It is a convenience
counter only — it is not decremented on cancellation or expiry.

## Functions

| Function | Signature required | Notes |
|---|---|---|
| `initialize(admin)` | admin | Once. |
| `set_registry(admin, registry)` | admin | Once, before the first plan. |
| `create_plan(merchant, name, token, amount_per_period, interval_ledgers, default_cap, description) -> u64` | **merchant** | Active immediately. `description` is `Option<String>`; if `Some`, it must be 1..=`MAX_DESCRIPTION_LEN` bytes. Counted in the registry if linked. |
| `set_plan_description(merchant, plan_id, description)` | **the plan's merchant** | Replaces the description (`None` clears it). Emits `plan_description_updated`. Same byte limit as `create_plan`. |
| `deactivate_plan(merchant, plan_id)` | **the plan's merchant** | Irreversible. |
| `get_plan(plan_id) -> Plan` | none | |
| `get_plan_subscriber_count(plan_id) -> u64` | none | Returns `Plan.subscriber_count`; 0 for unknown plans. |
| `get_plans_by_merchant(merchant) -> Vec<Plan>` | none | Active and inactive, oldest first. |
| `get_plans_by_token(token, offset, limit) -> Vec<Plan>` | none | Oldest first. If `offset >= len`, returns an **empty vec** (not an error); see below. |
| `get_active_plans() -> Vec<Plan>` | none | Oldest first. |
| `get_registry() -> Option<Address>` | none | |

### `get_plans_by_token` pagination

`get_plans_by_token` returns at most `limit` plans starting at `offset`. An
`offset` greater than or equal to the number of plans for that token is **not**
an error: the call returns an empty `Vec`. Callers that need to distinguish
"no plans exist for this token" from "offset is past the end" should first
call `get_plans_by_merchant` (or an indexer) to learn the total count, or
simply stop paging when an empty page is returned.

## What deactivation does and does not do

- New `subscribe` calls naming the plan fail with `PlanInactive`.
- Existing subscriptions keep charging exactly as authorized. The subscriber
  consented to specific terms and a cap; the merchant retiring a plan does not
  change that consent. A merchant who wants to stop billing simply stops
  calling `charge`.

This is covered by `deactivating_a_plan_leaves_existing_subscriptions_chargeable`
in the subscription crate's integration tests.

## Events

| Event | Topics | Data |
|---|---|---|
| `plan_created` | plan_id, merchant | name, token, amount_per_period, interval_ledgers, default_cap, description |
| `plan_description_updated` | plan_id, merchant | description |
| `plan_deactivated` | plan_id, merchant | — |

## Errors

| Code | Name | Meaning |
|---|---|---|
| 1 | `AlreadyInitialized` | |
| 2 | `NotInitialized` | |
| 3 | `Unauthorized` | Caller is not the admin. |
| 4 | `InvalidName` | Empty or longer than 64 bytes. |
| 5 | `InvalidAmount` | `amount_per_period <= 0`. |
| 6 | `InvalidInterval` | `interval_ledgers == 0`. |
| 7 | `InvalidCap` | `default_cap < amount_per_period`. |
| 8 | `PlanNotFound` | |
| 9 | `NotPlanMerchant` | Only the creating merchant may deactivate. |
| 10 | `PlanAlreadyInactive` | |
| 11 | `Overflow` | |
| 12 | `AlreadyConfigured` | Registry already linked, or a plan already exists. |
| 13 | `RegistryUpdateFailed` | The linked registry refused to count the plan. |
| 15 | `InvalidDescription` | Description is empty or longer than `MAX_DESCRIPTION_LEN` (256) bytes. |

Codes are append-only. (14 is reserved.)

## Limits

`get_active_plans` walks every plan ever created. For large catalogues, serve
listings from an indexer reading `plan_created` / `plan_deactivated` events.

`MAX_DESCRIPTION_LEN` is **256 bytes**. Descriptions are stored as UTF-8 and
the limit is measured in bytes, not characters.
