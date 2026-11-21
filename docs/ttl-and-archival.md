# State archival and TTL

Soroban does not keep storage entries alive forever. Every entry has a
time-to-live (TTL), measured in ledgers, and once it runs out the entry is
archived (evicted) and can no longer be read without being explicitly
restored. All three contracts (`registry`, `plan`, `subscription`) bump the
TTL of every entry they touch so that normal use keeps data alive, and this
document explains the constants involved and what happens if nobody touches
a subscription for a long time.

## Instance vs persistent entries

Each contract's `storage.rs` uses two kinds of entries, both bumped with the
same constants:

```rust
const DAY_IN_LEDGERS: u32 = 17_280;

const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;
```

(identical in `contracts/subscription/src/storage.rs`,
`contracts/registry/src/storage.rs` and `contracts/plan/src/storage.rs`.)

- **Instance storage** holds data there is only one of per contract:
  `Admin`, `NextId`, linked contract addresses, `Writers`, `Stats`. It is
  bumped to live 7 days (`INSTANCE_BUMP_AMOUNT`) whenever
  `bump_instance(env)` runs, once its remaining TTL falls to
  `INSTANCE_LIFETIME_THRESHOLD` (6 days) or below.
- **Persistent storage** holds per-id records that must survive
  independently of each other: a `Subscription`, a `Plan`, a
  `SubscriptionRecord`, and the index entries (`BySubscriber`, `ByMerchant`,
  `AllSubscriptions`, `AllPlans`). Each is bumped to live 30 days
  (`PERSISTENT_BUMP_AMOUNT`) via `bump_persistent(env, key)`, once its
  remaining TTL falls to `PERSISTENT_LIFETIME_THRESHOLD` (29 days) or below.

`DAY_IN_LEDGERS = 17_280` assumes a 5-second average ledger close time
(`86_400 / 5`).

## Why every read and write bumps TTLs

Every function that reads or writes a persistent entry calls
`bump_persistent` as part of that access:

- `read_sub` / `write_sub` (subscription), `read_plan` / `write_plan` (plan),
  `read_record` / `write_record` (registry) each bump the single entry they
  touch.
- `read_index` / `push_index` (subscription, plan) and `read_all_ids` /
  `push_id` (registry) bump the index entry they read or append to.

This means the TTL clock restarts on the *entry*, not on a per-subscription
basis across all its related entries. `charge` is the one entry point that
must bump more than its own record: it bumps `Sub(id)`, the subscriber's
`BySubscriber` index and the merchant's `ByMerchant` index (via
`touch_index`, which only extends an index that already exists), and the
registry's `Record(id)` when linked. Without that, a subscriber who is charged
for years but never calls the contract would let `BySubscriber` archive while
its records stay live, breaking `get_by_subscriber`, `cancel` and
`refresh_allowance`. Entry point functions that touch several of these keys
in one transaction bump all of the keys they go through.

Instance-level data is bumped the same way, via `bump_instance`, inside the
contracts' entry points that touch instance storage.

## What happens if nobody touches a subscription for a long time

If a `Subscription` entry's TTL reaches zero before anything reads or writes
it again, the entry is archived. From that point:

- Any call that would read it (`get_subscription`, `is_chargeable`,
  `remaining_cap`, `charge`, `cancel`, `pause`, `resume`,
  `refresh_allowance`) fails, because Soroban cannot return an archived
  entry's value without it first being restored.
- The subscription is not deleted and its data is not lost. It is simply
  inaccessible until restored.

Because `interval_ledgers` can be set far below 30 days, a subscription that
is being charged regularly keeps its own TTL extended far past the point
where archival would ever be reached. The scenario this section describes
applies to a subscription nobody has charged, cancelled, paused, resumed or
even queried for 30 days straight - which the contracts cannot prevent, since
they have no scheduler and only extend a TTL as a side effect of some
address calling in.

The `BySubscriber` / `ByMerchant` index entries for a subscriber or merchant
with no activity for 30 days are equally subject to archival, independently
of the `Sub(id)` entries they list.

## The registry's `AllSubscriptions` history

The registry keeps one persistent vec, `AllSubscriptions`, of every
registered subscription id. It is bumped only by `register_subscription`. If
no subscription is registered for the 30-day persistent window, the vec
archives and `get_all_subscriptions` returns an empty result. The next
registration then starts a new vec, so the earlier history is missing from the
registry's view until it is restored.

Mitigation: `keep_alive()` on the registry extends the TTL of
`AllSubscriptions` and the instance storage. It is permissionless and changes
no data. Call it at least every 20 days, for example from a scheduled job. The
test `history_survives_a_quiet_period_when_kept_alive` in
`contracts/registry/src/test.rs` covers a 60-day quiet period with that
cadence.

## Restoring an archived entry

Soroban allows an archived persistent entry to be restored (paying the
restoration fee) as long as it is within the network's maximum entry
lifetime of its eviction; beyond that window, restoration is not possible by
anyone. None of these contracts implement a restore path of their own - this
would be done with the network's standard restore operation, outside of any
contract call, before calling back into the contract. This repository has
no tooling for that today; it is a gap the contracts do not address, not a
documented workflow.
