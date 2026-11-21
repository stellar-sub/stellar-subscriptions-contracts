# Design doc: free trial via `trial_ledgers` (#53)

Status: retroactive. `trial_ledgers` already exists on the `Subscription`
type and is exercised by `contracts/subscription/src/test/new_features.rs`;
this doc documents the decision after the fact per issue #274.

## Problem

A merchant wants to let a subscriber sign up without being charged
immediately — a free trial of a fixed length before the first pull.

## Decision

Add `trial_ledgers: u32` to `Subscription` (0 means "no trial"). Rather than
adding a parallel `TrialActive` status, the trial is expressed purely through
`next_charge_ledger`: at creation it is set to `start_ledger + trial_ledgers`
instead of `start_ledger`. The subscription's `status` stays `Active` for the
whole trial.

Consequences of reusing `next_charge_ledger` instead of a new status:

- `charge` needs no trial-specific branch — its existing
  `next_charge_ledger` / `interval_ledgers` check already refuses a charge
  before the trial ends, and accepts one at or after it.
- `pause` / `resume` / `cancel` work unmodified during a trial, since they
  only inspect `status`, which is `Active`.
- `charge_status` reports the same "not due yet" reason it would for any
  future `next_charge_ledger`, without a dedicated "in trial" variant.

The cost is that nothing distinguishes "never charged because of trial" from
"never charged because the interval has not elapsed yet" in the status enum
— callers that need to show "trial" in a UI read `trial_ledgers > 0 &&
total_charged == 0` instead of a status value.

## Interaction with other fields

- `trial_ledgers` only affects the *first* charge. `last_charge_ledger`
  stays 0 until that first charge, exactly as for a non-trial subscription.
- `total_cap` and `amount_per_period` are unaffected — a trial delays
  charging, it does not grant extra cap.
- A trial that is never reached (subscription cancelled or merchant-ended
  first) simply never produces a charge; no special-case cleanup is needed
  because cancellation/merchant-end do not look at `trial_ledgers`.

## Known gap at the time of this doc

As of this fork's `lib.rs`, `subscribe`'s parameter list does not yet take a
`trial_ledgers` argument — the field exists on the type and is asserted by
`new_features.rs`, but the constructor always writes `0`/`start_ledger`
implicitly via the existing `subscribe` signature. Wiring the parameter
through `subscribe` (or adding a `subscribe_with_trial` entry point, by the
same backward-compatibility reasoning as `subscribe_until` below) is tracked
separately and is not part of this documentation issue.
