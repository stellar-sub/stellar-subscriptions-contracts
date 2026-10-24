# FAQ

Short answers for subscribers, merchants and contributors. Each links to the
detailed doc for the full picture.

### 1. Why can't a merchant charge twice in a row?

`charge` schedules the next allowed charge from the ledger the charge
*actually* happened on (`next_charge_ledger = now + interval_ledgers`), and
rejects any call before that ledger with `IntervalNotElapsed`. See the
**INTERVAL** invariant in `docs/subscription.md`.

### 2. What does `pause` do?

It blocks charges and changes nothing else - no fields on the subscription
move except its status. See "Pause and resume" in `docs/subscription.md`.

### 3. Why did my charge fail?

Most commonly: the interval hasn't elapsed yet (`IntervalNotElapsed`), the
charge would exceed the cap (`CapExceeded`), the subscription isn't
`Active` (`SubscriptionPaused` / `SubscriptionCancelled` /
`SubscriptionExhausted`), it has an `expires_at` that has passed
(`SubscriptionExpired`), or the token refused the pull - insufficient
balance or a stale allowance (`TransferFailed`). A failed charge changes no
state, so nothing is lost; see "`charge()` in detail" in
`docs/subscription.md`.

### 4. Why would the token refuse the pull even though I subscribed?

The contract's allowance (what the token lets it pull) is only recomputed
on `subscribe`, `cancel` and `refresh_allowance`, and it expires. If it
expired or drifted, call `refresh_allowance` to re-approve it. See
`docs/allowance-model.md`.

### 5. Can I have more than one subscription in the same token?

Yes. The allowance is the sum of unspent cap across all of a subscriber's
non-final subscriptions in that token, recomputed whenever it changes - it
is not overwritten per subscription. See the worked example in
`docs/allowance-model.md`.

### 6. What happens to my allowance when I cancel one subscription?

Only that subscription's unused cap is released. The allowance is
recomputed across the remaining non-final subscriptions in that token, so
any other subscription's share is untouched. See `docs/allowance-model.md`.

### 7. Is cancelling reversible?

No. `Cancelled` is final - the **REVOCATION** invariant guarantees no
charge ever succeeds again after cancellation, and nothing moves a
subscription out of `Cancelled`. See `docs/subscription.md`.

### 8. What happens if I pause, then come back after my next charge was due?

Resuming sets `next_charge_ledger = max(next_charge_ledger, now)`: if the
due date already passed while paused, exactly one charge becomes available
immediately; if it hadn't passed yet, the original schedule is kept.
Paused periods are never billed retroactively. See "Pause and resume" in
`docs/subscription.md`.

### 9. Can a merchant deactivating a plan affect my existing subscription?

No. Deactivating a plan only blocks *new* subscriptions from choosing it.
It has no effect on subscriptions already created from it - what the
subscriber authorized governs, not the plan's current state. See "What
deactivation does and does not do" in `docs/plan.md`.

### 10. Can the admin cancel, pause or charge my subscription?

No. The admin can only link the plan and registry contracts, once, before
the first subscription exists. It has no authority over any individual
subscription. See "Who may call what" in `docs/subscription.md`.

### 11. Why does `subscribe` sometimes fail with `PlanMismatch`?

Subscribing with a non-zero `plan_id` requires the merchant, token, amount
and interval to exactly match the plan's terms; the subscriber still picks
their own `total_cap`. See "Plans" in `docs/subscription.md`.

### 12. What happens if my subscription just sits there, unused, for a long time?

Its storage entry's TTL eventually runs out and it is archived - it is not
deleted, but it becomes unreadable until explicitly restored. Routine
activity (charging, cancelling, pausing, resuming, or even querying) resets
the TTL as a side effect. See `docs/ttl-and-archival.md`.

### 13. Can an existing deployed contract be upgraded in place?

No - none of the three contracts expose an upgrade entry point. Changing
behavior means deploying new instances and linking them together; existing
instances and their data are left exactly as they are. See
`docs/versioning-and-upgrades.md`.

### 14. Is it safe for a client to match on numeric error or status codes?

Yes, as long as the client only expects codes it has already seen or new
ones it's prepared to treat as "an error I don't specifically handle" -
error and status codes are append-only and never renumbered or reused. See
`docs/versioning-and-upgrades.md`.

### 15. Does the registry decide whether a charge is allowed?

No. The registry is bookkeeping only - it never moves funds and has no say
over whether `charge`, `cancel`, `pause` or `resume` succeed. A broken
registry can make those calls fail (hurting availability), but it can never
make a charge succeed that the contract's own invariants would reject. See
"Caveats" in `docs/registry.md` and "Trust assumptions and limits" in
`docs/subscription.md`.
