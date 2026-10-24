# Versioning, upgrades and migrations

None of the three contracts (`registry`, `plan`, `subscription`) expose an
upgrade or migration entry point. There is no admin function that replaces a
contract's Wasm or rewrites its storage layout. This document describes what
that means in practice, and what is and is not safe to change about a
deployed instance.

## What can change without breaking clients

- **New error codes.** `Error` enums in each contract
  (`contracts/*/src/errors.rs`) are append-only: a new failure can be added
  with the next unused discriminant. Existing codes are never renumbered or
  reused, so a client matching on a numeric code keeps working.
- **New `SubStatus` discriminants** would follow the same append-only rule,
  though none have been added since `Active = 0`, `Paused = 1`,
  `Cancelled = 2`, `Exhausted = 3` were fixed (`contracts/subscription/src/types.rs`).
- **New events or new data fields on an event**, as long as existing topics
  and existing fields keep their meaning.
- **New read-only view functions.**
- **Deploying a new instance** of any contract on a different address. Since
  there is no shared global state between instances, a fresh deployment never
  affects an existing one.

## What cannot change

- **Removing or renumbering an existing error code.** Status and error
  discriminants are part of the public interface: the registry and
  off-chain indexers decode `status` as a raw `u32`
  (`contracts/registry/src/types.rs`), so a renumbering silently corrupts
  every consumer that is not redeployed in lockstep.
- **Changing the shape of a stored type** (`Subscription`, `Plan`,
  `SubscriptionRecord`, ...) in a way that is not purely additive. Every
  value already written to persistent storage was encoded with the old
  shape; a non-additive change makes existing entries fail to decode the
  next time they are read.
- **Changing a contract's code at its existing address.** None of these
  contracts implement an upgrade entry point (there is no
  `update_current_contract_wasm` call or similar in any `lib.rs`), so this is
  not a supported operation at all, not just a discouraged one.
- **Changing `Plan`'s field names.** The subscription contract reads `Plan`
  across contracts and decodes it by field name (`docs/plan.md`), so a rename
  breaks that cross-contract read.

## What "deploy fresh instances" means for the links

The subscription, plan and registry contracts are linked by address
(`set_plan_contract`, `set_registry` on the subscription contract;
`set_registry` on the plan contract; `set_writers` on the registry), and
every link is admin-set exactly once, before the first subscription or plan
is created. There is no way to repoint an existing instance's link once set.

Consequently, a "migration" in this system is not an in-place upgrade: it is
deploying new instances of the changed contract(s) and linking them together
from scratch. Existing instances, and any subscriptions or plans already
recorded in them, are left exactly as they are; they are not moved or
converted. A registry only ever reflects what was registered with it, so
plans and subscriptions created before a registry was linked, or created
against a different registry instance entirely, are simply absent from it
(`docs/registry.md`).

## Not supported yet

- No contract upgrade mechanism (Wasm hash replacement) exists for any of the
  three contracts.
- No data migration tooling exists to move subscriptions, plans or registry
  records from an old instance to a new one. Carrying state forward across a
  redeploy would have to be done by an off-chain process replaying history
  from events, not by any function in these contracts.
- No mechanism exists to repoint `plan_contract`, `registry` or `writers`
  after they are first set.
