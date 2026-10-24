# Glossary

Terms used across these docs and in the authorization text a wallet shows
when signing a call into these contracts.

## Ledger

Stellar's unit of consensus: one closed block of transactions, numbered
sequentially. The contracts measure every duration — a billing interval, an
allowance's expiry, a subscription's optional end — in ledgers rather than
wall-clock time, because `env.ledger().sequence()` is what the contract can
actually read.

**Ledger time is an estimate, not a guarantee.** Ledgers close roughly every
5 seconds, so a duration of `N` ledgers is *about* `N * 5` seconds, but
network conditions can make individual ledgers close faster or slower. Never
treat a ledger count as an exact clock. See `interval_ledgers` in
[docs/subscription.md](subscription.md#the-three-invariants) and
`created_at` in [docs/plan.md](plan.md#data).

## Stroop

The smallest unit of a Stellar asset: 1 XLM = 10,000,000 stroops. Contract
amounts (`amount_per_period`, `total_cap`, token balances) are always in an
asset's smallest unit — stroops for XLM, or the equivalent smallest unit for
any other token — never in whole XLM. See the example plan in
[DEPLOYMENTS.md](../DEPLOYMENTS.md) ("1 XLM (10,000,000 stroops) every 720
ledgers").

## TTL (time to live)

How many more ledgers a piece of contract storage will survive before the
network is allowed to archive it. Soroban storage entries must be
periodically "bumped" (extended) or they expire and have to be restored
before they can be read or written again. `bump_instance` and
`bump_persistent` in each contract's `storage.rs` extend the TTL of instance
and persistent storage on every write, so day-to-day contract data does not
expire. See `extend_ttl` calls in `contracts/*/src/storage.rs`.

## Allowance

A Soroban token's built-in approval: an amount a spender (this contract) may
pull from an owner's (the subscriber's) balance, until an expiry ledger. A
subscription contract uses `transfer_from` against this allowance to
charge, because a pull payment cannot carry the subscriber's live signature
at charge time. The allowance is **defence in depth**, not the enforcement —
the CAP, INTERVAL and REVOCATION invariants hold even if the allowance were
larger than it should be. See
[docs/subscription.md § Token allowance](subscription.md#token-allowance).

## Total cap

The most a subscription can ever charge, across its whole lifetime, in the
units of its token (stroops, if the token is XLM). Set once at `subscribe`
and never increased. See the **CAP** invariant in
[docs/subscription.md](subscription.md#the-three-invariants).

## Interval (ledgers)

The minimum number of ledgers that must pass between two charges on the same
subscription. Measured in ledgers, so it is a ledger-time estimate, not an
exact wall-clock duration. See the **INTERVAL** invariant in
[docs/subscription.md](subscription.md#the-three-invariants).

## Authorization (`require_auth`)

A Soroban contract call proves who it is acting on behalf of: each
state-changing function requires one specific party's signature, checked
with `require_auth()`. A wallet's confirmation screen before signing is the
UI's authorization text for this — for example, `charge` always requires the
**merchant's** signature, and `cancel`, `pause`, `resume` and
`refresh_allowance` always require the **subscriber's**. See
[docs/subscription.md § Who may call what](subscription.md#who-may-call-what).

## Revocation

The subscriber's irreversible withdrawal of consent: once `cancel` succeeds,
no charge on that subscription ever succeeds again, and nothing — not the
merchant, the admin, the token or the registry — can block or undo it. See
the **REVOCATION** invariant in
[docs/subscription.md](subscription.md#the-three-invariants).

## Plan

A merchant-published template of billing terms (token, amount, interval, a
suggested cap) that a subscriber can subscribe to instead of entering terms
by hand. A plan never moves funds and has no authority over subscriptions
made from it. See [docs/plan.md](plan.md).

## Registry

A bookkeeping-only index of plans and subscriptions with aggregate stats,
for dashboards and indexers to read without replaying every event. It has no
say in whether a charge is allowed. See [docs/registry.md](registry.md).

## Wasm

WebAssembly: the compiled bytecode format Soroban contracts run as on the
Stellar network. `make build` / `make wasm` compile these Rust contracts to
`.wasm` for deployment.
