# Design doc: merchant-initiated termination (#54)

Status: retroactive. `SubStatus::MerchantEnded` and the `MerchantEnded` event
already exist; this doc documents the decision after the fact per issue
#274.

## Problem

Cancellation (`cancel`) is subscriber-only. Some merchants need to end a
subscription themselves — e.g. the subscriber's account was closed, or the
underlying plan was discontinued — without being able to grant themselves
further charges first.

## Decision

Add a fifth, terminal `SubStatus::MerchantEnded`, distinct from
`Cancelled`:

- Both are final (`SubStatus::is_final()` includes both), so `charge`
  refuses either identically.
- Keeping them distinct (rather than reusing `Cancelled`) preserves the
  meaning of existing `Cancelled` events and status reads: "the subscriber
  chose to stop" is not the same fact as "the merchant stopped it", and
  downstream accounting/support tooling needs to tell them apart.

`merchant_end(env, merchant, subscription_id)` is designed as the mirror of
`cancel`:

- Requires `merchant.require_auth()` and that `merchant` matches
  `sub.merchant` (an `Error::NotMerchant`-style check), the same way `cancel`
  requires the caller to be `sub.subscriber`.
- Allowed only from `Active` or `Paused`, refusing with
  `SubscriptionCancelled` / `SubscriptionExhausted` / a new
  `SubscriptionMerchantEnded` error if already final — matching `cancel`'s
  own match arms.
- Performs the same best-effort allowance sync and registry report as
  `cancel`, since both are "stop future charges" operations and the cleanup
  has nothing to do with who requested it.
- Emits `MerchantEnded { subscription_id, merchant, total_charged }` instead
  of `Cancelled`, topic-indexed by `merchant` rather than `subscriber`, so a
  merchant can filter their own terminations without scanning subscriber
  topics.

Deliberately out of scope: a merchant can only stop future charges, never
retroactively reverse a past one — `total_charged` is reported, not altered,
consistent with every other terminal transition in this contract.

## Known gap at the time of this doc

As of this fork's `lib.rs`, the `merchant_end` function itself (and the
`Error::SubscriptionMerchantEnded` variant it needs) is not yet implemented,
even though the `SubStatus::MerchantEnded` variant, its `is_final()` arm, and
the `MerchantEnded` event struct are already in place. Tests in
`contracts/subscription/src/test/new_features.rs` call
`s.client.merchant_end(...)` against this design; wiring the function and
the pause/resume/cancel match arms it affects is tracked separately from
this documentation issue.
