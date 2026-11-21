# Design doc: `subscribe_until` / `expires_at` (#52 follow-up)

Status: retroactive. This finalizes the proposal in `docs/design/52-expires-at.md`
by recording which option was picked and why, per issue #274.

## Decision

Of the two options `52-expires-at.md` laid out, **add a separate
`subscribe_until` entry point** rather than changing `subscribe`'s
signature:

- `subscribe` keeps its existing parameter list and behavior exactly,
  including for every existing caller and every ABI already deployed against
  it — no major-version bump under `version()`.
- `subscribe_until(env, subscriber, merchant, token, amount_per_period,
  interval_ledgers, total_cap, plan_id, expires_at: u32)` forwards to the
  same internal constructor as `subscribe`, passing `expires_at` through,
  with `0` meaning "no expiry" (matching the `trial_ledgers` convention of
  0 = feature off).

`expires_at` is a field on `Subscription`, not a derived value, because it
must survive exactly as given: it is a ledger number the subscriber agreed
to, and recomputing anything from current state at `charge` time would
reintroduce the same kind of simulation/submission drift that
`simulation_drift.rs` guards against elsewhere in this contract (see
`docs/allowance-model.md`).

## Enforcement point

`charge` gains one more refusal: once `env.ledger().sequence() > expires_at`
(when `expires_at != 0`), it refuses with a new `Expired`-style error before
doing any transfer — same position as the existing interval/cap/status
checks, so expiry never partially charges. `charge_status` reports the same
reason as its own "would fail" case, consistent with how it already reports
interval and cap failures without actually calling `charge`.

Expiry is deliberately **not** a `SubStatus` variant: it does not need
`pause`/`resume`/`cancel` to special-case it, because those only ever read
`status`. A subscription past its `expires_at` is still `Active` in status
but simply always fails `charge`'s expiry check — the same pattern
`trial_ledgers` uses for delaying rather than gating via a status.

## Known gap at the time of this doc

As of this fork's `lib.rs` and `types.rs`, neither `subscribe_until` nor the
`expires_at` field exists yet — `contracts/subscription/src/test/expiry.rs`
is written against this design and currently has nothing to call. Wiring
`expires_at` onto `Subscription` and adding `subscribe_until` plus the
`charge` expiry check is tracked separately from this documentation issue.
