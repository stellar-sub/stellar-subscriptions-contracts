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
use common::DAY_IN_LEDGERS; // contracts/common: 17_280

const INSTANCE_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;
```

(the subscription, registry and plan contracts all import `DAY_IN_LEDGERS`
from `contracts/common`, so the day length is defined once. The instance
constants are set per contract; the values above are the subscription
contract's, which is the one that must stay live between charges.)

- **Instance storage** holds data there is only one of per contract:
  `Admin`, `NextId`, linked contract addresses, `Writers`, `Stats`. It is
  bumped to live 30 days (`INSTANCE_BUMP_AMOUNT`) whenever
  `bump_instance(env)` runs, once its remaining TTL falls to
  `INSTANCE_LIFETIME_THRESHOLD` (29 days) or below. The 30-day bump is what
  lets a deployment with only monthly subscriptions go a full billing cycle
  without a contract call and still accept the next `charge`.
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
basis across all its related entries: charging a subscription bumps its
`Sub(id)` entry, but does not by itself bump that subscriber's `BySubscriber`
index entry unless that index is also read or written in the same call.
Entry point functions that touch several of these keys in one transaction
(e.g. `charge`, which reads `Sub(id)` and may update the registry's
`Record(id)`) bump all of the keys they go through.

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

## Restoring an archived entry

Soroban allows an archived persistent entry to be restored, for a fee, as
long as it is within the network's maximum entry lifetime of its eviction.
Beyond that window it cannot be restored by anyone.

None of these contracts have a restore entry point, and they cannot add one
that works. Restoration is a ledger operation: it is performed in a
transaction before the contract is called, not from inside a contract call,
so a `restore_subscription(id)` function would have no way to bring back an
evicted `Sub(id)`. Recovery is done with the Stellar CLI.

### Procedure

1. Confirm the entry is archived. A read of an archived `Sub(id)` fails:

   ```bash
   stellar contract invoke --id $SUBSCRIPTION_CONTRACT --source merchant \
     -- get_subscription --subscription_id $SUB_ID
   ```

   An archived entry is reported as an error from the simulation, which
   names the missing or archived key.

2. Restore the persistent entry with `stellar contract restore`. Pass the
   contract id, the key, and the persistent durability:

   ```bash
   stellar contract restore --id $SUBSCRIPTION_CONTRACT \
     --durability persistent \
     --key '<KEY for Sub(SUB_ID)>'
   ```

   Replace `<KEY for Sub(SUB_ID)>` with the key of the subscription record.
   The key is the `DataKey::Sub(u64)` value, so it is written as an enum
   variant with the subscription id. Check the exact key encoding against
   `stellar contract restore --help` for your CLI version before running
   this, because the argument format for enum keys differs between releases.

3. Repeat the read from step 1. It should now return the subscription.

If the restore is refused, the entry is past the maximum restoration window
and cannot be recovered.

### Prevention

Charging, pausing, resuming and cancelling each extend the instance and the
touched entries, so a subscription that is used regularly does not archive.
To guard a deployment with long billing cycles, call a read-only entry point
such as `get_subscription` at least once a month for each subscription, or
run `charge` on each billing date.
