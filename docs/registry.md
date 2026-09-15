# Registry contract

`contracts/registry` — an index of plans and subscriptions with aggregate
stats, for dashboards and indexers.

The registry is **bookkeeping only**. It never moves funds and has no say in
whether a charge is allowed. It exists so a UI can show totals and lists
without replaying every event.

## Writers

Only two contracts may write, and they are fixed once:

- the **subscription contract** may register and update subscriptions;
- the **plan contract** may register plans.

`set_writers` is admin-only and callable exactly once, so the admin cannot
later point the registry at a contract that rewrites history. Anyone else
calling a write function fails authorization.

## Data

```rust
pub struct RegistryStats {
    pub total_plans: u64,
    pub total_subscriptions: u64,
    pub active_subscriptions: u64,   // last reported status is Active
    pub total_charged_volume: i128,  // sum of charges, raw units, all tokens
}

pub struct SubscriptionRecord {
    pub id: u64,
    pub subscriber: Address,
    pub merchant: Address,
    pub total_charged: i128,
    pub status: u32,                 // 0 Active, 1 Paused, 2 Cancelled, 3 Exhausted
}
```

## Functions

| Function | Caller | Notes |
|---|---|---|
| `initialize(admin)` | admin | Once. |
| `set_writers(admin, subscription_contract, plan_contract)` | admin | Once. |
| `register_plan(plan_id)` | plan contract | Each plan counted once. |
| `register_subscription(subscription_id, subscriber, merchant)` | subscription contract | Starts Active. |
| `update_subscription(subscription_id, total_charged, status)` | subscription contract | See rules below. |
| `get_stats() -> RegistryStats` | anyone | |
| `get_subscription_record(subscription_id)` | anyone | |
| `get_all_subscriptions() -> Vec<u64>` | anyone | Oldest first. |
| `get_active_subscriptions() -> Vec<u64>` | anyone | Oldest first. |
| `get_writers() -> Writers` | anyone | |

## Update rules

`update_subscription` mirrors the subscription contract's own rules so the
index can never reach a state the subscription contract could not:

- `status` must be 0–3 (`InvalidStatus`);
- `total_charged` never decreases (`ChargedDecreased`);
- a Cancelled or Exhausted record never changes again (`AlreadyFinal`);
- `total_charged_volume` grows by exactly the increase, with checked
  arithmetic;
- `active_subscriptions` moves only on a real transition into or out of
  Active, so cancelling a paused subscription is not double-counted.

## Caveats

- `total_charged_volume` adds raw token units across **all tokens**. It is a
  activity signal, not a currency amount. Per-token volume belongs in an
  indexer.
- `get_active_subscriptions` walks every registered subscription.
- Plans and subscriptions created before the registry was linked would be
  missing, which is why both the plan and subscription contracts only accept
  a registry link before their first record.

## Errors

| Code | Name |
|---|---|
| 1 | `AlreadyInitialized` |
| 2 | `NotInitialized` |
| 3 | `Unauthorized` |
| 4 | `NotConfigured` |
| 5 | `AlreadyConfigured` |
| 6 | `SubscriptionAlreadyRegistered` |
| 7 | `SubscriptionNotFound` |
| 8 | `InvalidStatus` |
| 9 | `ChargedDecreased` |
| 10 | `AlreadyFinal` |
| 11 | `Overflow` |
| 12 | `PlanAlreadyRegistered` |

Codes are append-only.
