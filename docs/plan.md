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
}
```

The subscription contract reads `Plan` across contracts and decodes it by
field name, so **field names must not change**.

## Functions

| Function | Signature required | Notes |
|---|---|---|
| `initialize(admin)` | admin | Once. |
| `set_registry(admin, registry)` | admin | Once, before the first plan. |
| `create_plan(merchant, name, token, amount_per_period, interval_ledgers, default_cap) -> u64` | **merchant** | Active immediately. Counted in the registry if linked. |
| `deactivate_plan(merchant, plan_id)` | **the plan's merchant** | Stops new subscriptions; reversible via `reactivate_plan`. |
| `reactivate_plan(merchant, plan_id)` | **the plan's merchant** | Re-enables a deactivated plan, keeping its ID. |
| `get_plan(plan_id) -> Plan` | none | |
| `get_plans_by_merchant(merchant) -> Vec<Plan>` | none | Active and inactive, oldest first. |
| `get_plans_by_merchant_page(merchant, start, limit) -> Vec<Plan>` | none | Paginated variant of `get_plans_by_merchant`. |
| `get_active_plans_by_merchant(merchant) -> Vec<Plan>` | none | Only active plans, oldest first. |
| `get_active_plans() -> Vec<Plan>` | none | Oldest first. **Deprecated** in favour of `get_active_plans_page`. |
| `get_active_plans_page(start, limit) -> Vec<Plan>` | none | Paginated active plans, oldest first. |
| `get_registry() -> Option<Address>` | none | |

## What deactivation does and does not do

- New `subscribe` calls naming the plan fail with `PlanInactive`.
- `reactivate_plan` restores `active = true` on the same plan ID, so
  subscriptions can resume without creating a new plan.
- Existing subscriptions keep charging exactly as authorized. The subscriber
  consented to specific terms and a cap; the merchant retiring a plan does not
  change that consent. A merchant who wants to stop billing simply stops
  calling `charge`.

This is covered by `deactivating_a_plan_leaves_existing_subscriptions_chargeable`
in the subscription crate's integration tests.

## Events

| Event | Topics | Data |
|---|---|---|
| `plan_created` | plan_id, merchant | name, token, amount_per_period, interval_ledgers, default_cap |
| `plan_deactivated` | plan_id, merchant | — |
| `plan_reactivated` | plan_id, merchant | — |

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
| 9 | `NotPlanMerchant` | Only the creating merchant may deactivate or reactivate. |
| 10 | `PlanAlreadyInactive` | |
| 11 | `Overflow` | |
| 12 | `AlreadyConfigured` | Registry already linked, or a plan already exists. |
| 13 | `RegistryUpdateFailed` | The linked registry refused to count the plan. |
| 14 | `PlanAlreadyActive` | `reactivate_plan` called on an active plan. |
| 15 | `InvalidPageSize` | `limit` is 0 or greater than `MAX_PAGE_SIZE`. |

Codes are append-only.

## Limits

`get_active_plans` walks every plan ever created and will exceed the
per-transaction instruction budget for large catalogues. Prefer the paginated
queries (`get_active_plans_page`, `get_plans_by_merchant_page`) with a `limit`
of at most `MAX_PAGE_SIZE`. For bulk listings, serve them from an indexer
reading `plan_created` / `plan_deactivated` / `plan_reactivated` events.
