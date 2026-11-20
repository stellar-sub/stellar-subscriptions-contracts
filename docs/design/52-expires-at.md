# Design proposal: optional end ledger for subscriptions (#52)

Status: implemented. The design below was agreed and shipped: `Subscription`
now carries `expires_at: u32` (0 means no expiry), `subscribe_until` is the
entry point that sets it, `charge` refuses with `Error::SubscriptionExpired`
once the current ledger passes a non-zero `expires_at`, and the `Subscribed`
event carries the field.

## Problem

A subscriber may want "for one year only". Today a subscription runs until its
cap is reached or it is cancelled.

## Proposal

Add an end ledger, `expires_at: u32`, to the subscription, where `0` means no
expiry. After that ledger `charge` is refused. The expiry can only narrow what
the merchant may do, so it never lets a merchant charge more.

Because `subscribe` takes this as a new argument, the proposal offers two ways
to add it:

- **Change `subscribe`.** Add the argument to `subscribe`. This is the simplest
  API but breaks every caller, which is a major change under `version()`.
- **Add `subscribe_until`.** Keep `subscribe` unchanged and add a second entry
  point that takes the end ledger. No caller breaks. This is the recommended
  option.

Either way:

- `charge` refuses with `Error::SubscriptionExpired` once the current ledger
  passes a non-zero `expires_at`, and `charge_status` reports it as its own
  reason.
- A subscription with no expiry (`expires_at == 0`) behaves exactly as today.
- The `Subscribed` event includes `expires_at`, and the golden XDR test covers
  it.

## Trade-offs

- A second entry point means two ways to create a subscription. Both must share
  one code path, or they can drift.
- Adding a field to `Subscription` changes the stored layout, so `VERSION` must
  be bumped in either option.

## Resolved questions

- The expiry is a ledger number, as proposed, not a duration from subscription
  time.
- `subscribe_until` was accepted; `subscribe` is unchanged and sets
  `expires_at` to `0`.
