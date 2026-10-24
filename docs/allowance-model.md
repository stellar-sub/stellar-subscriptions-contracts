# The allowance model, explained for non-experts

This document explains `contracts/subscription/src/allowance.rs` without
assuming familiarity with Soroban tokens.

## Why this exists at all

A regular token transfer needs the sender to sign it at the moment it
happens. But a subscription is a *pull* payment: the merchant, not the
subscriber, triggers each charge, and the subscriber is not online to sign
anything at that moment.

Soroban tokens solve this the same way ERC-20 does: the subscriber signs a
one-time `approve`, naming this contract as a spender and a maximum amount
it may pull. From then on, the contract can call `transfer_from` to move
money out of the subscriber's account, up to that approved amount, without
the subscriber signing again.

So "the allowance" is just that approved amount, as tracked by the token
contract. This contract's job is to keep it set to the right number.

## The steps

### 1. Subscribe approves

When a subscriber calls `subscribe`, the contract calls `sync`, which:

1. Computes `outstanding`: the sum of "cap still chargeable" across every
   one of that subscriber's subscriptions **in that same token** that is not
   already `Cancelled` or `Exhausted`. For one subscription, that is
   `total_cap - total_charged`.
2. Approves this contract to spend exactly that amount, until an expiry
   ledger (see "why the expiry is aligned" below).

### 2. Charge pulls

`charge` does not touch the allowance at all. It calls the token's
`transfer_from(subscriber, merchant, amount_per_period)`, which the token
only allows because of the approval from step 1. The allowance the token
contract tracks for this subscriber/spender pair goes down automatically,
by whatever the token does internally - this contract never reads or writes
that number directly between `sync` calls.

### 3. Cancel lowers

Cancelling a subscription makes it `Cancelled`, which `outstanding` excludes
from its sum (cancelled subscriptions are final: no more charges). `cancel`
calls `sync` again, which recomputes `outstanding` across the subscriber's
*remaining* non-final subscriptions in that token and re-approves to exactly
that lower number. The subscriber's unused cap on the cancelled subscription
stops being part of what this contract may ever pull.

### 4. Expiry aligns to a window

Approvals are not forever - they carry an expiry ledger. Recomputing the
allowance also means recomputing that expiry, pushing it back out to the
network's maximum each time `sync` runs (on `subscribe`, `cancel`, and the
subscriber-callable `refresh_allowance`).

## Worked example: two subscriptions in one token

Subscriber Alice, token USDC, merchant Bob and merchant Carol.

1. Alice calls `subscribe` to Bob: `amount_per_period = 10`,
   `total_cap = 100`. This is her only USDC subscription so far.
   `outstanding = 100 - 0 = 100`. The contract approves **100 USDC**.
2. Alice calls `subscribe` to Carol: `amount_per_period = 5`,
   `total_cap = 50`. Now there are two non-final USDC subscriptions:
   Bob's (`100 - 0 = 100`) and Carol's (`50 - 0 = 50`).
   `outstanding = 100 + 50 = 150`. The contract approves **150 USDC**,
   replacing the earlier 100. This is the "adds to the allowance instead of
   overwriting the first" behavior: the second `subscribe` does not ignore
   the first subscription, it recomputes the total across both.
3. Bob charges once: `total_charged` on Bob's subscription becomes 10.
   Nothing calls `sync` here, so the token's own allowance simply drops by
   10 USDC to 140 USDC as a side effect of `transfer_from` - this contract's
   bookkeeping is untouched.
4. Alice cancels Bob's subscription. `cancel` calls `sync`. Bob's
   subscription is now `Cancelled`, so `outstanding` only counts Carol's:
   `50 - 0 = 50`. The contract approves **50 USDC**, overwriting whatever
   the token's allowance happened to be at that point (it does not matter
   that step 3 already lowered it to 140 - `sync` always sets the allowance
   to the freshly computed number, not a relative adjustment). Carol's share
   is untouched; only Bob's unused 90 USDC of cap is released.

If the token's allowance ever drifts from what this contract expects (for
example, if a charge fails and the token state in step 3 is not what was
assumed), the next `sync` call - from any `subscribe`, `cancel`, or
`refresh_allowance` - repairs it, because it is always recomputed from
scratch rather than incremented or decremented.

## Why the expiry is aligned, and what a window boundary does

A wallet signs the `approve` call with the exact arguments it saw while
simulating the transaction, including the expiry ledger. The transaction
then lands a few ledgers later than the simulation. If the expiry were
computed from "now" at simulation time, by the time the transaction landed
"now" would have moved on and the expiry baked into the signed call would no
longer equal what submission computes - so the signature would not match,
and the call would fail on submission after having passed simulation. This
is not hypothetical: `allowance.rs`'s doc comment notes it happened on
Testnet before the fix.

The fix is to round the expiry down to the start of a fixed window
(`EXPIRY_WINDOW = 17_280` ledgers, about a day) before adding the maximum
TTL:

```rust
let window_start = now - now % EXPIRY_WINDOW;
window_start.saturating_add(env.storage().max_ttl()).max(now)
```

Within one window, "now" rounds down to the same `window_start` no matter
when exactly simulation or submission happens, so the expiry computed at
simulation time matches the expiry computed at submission time, and the
signature stays valid.

The only case this does not cover is when a window boundary falls *between*
simulation and submission - the two calls would then round down to
different `window_start` values and disagree on the expiry. That call fails
cleanly with `ApprovalFailed` (no state changes) and can simply be retried.
`contracts/subscription/src/test/simulation_drift.rs` tests `subscribe`,
`cancel` and `refresh_allowance` signed several ledgers before they land, to
cover exactly this.
