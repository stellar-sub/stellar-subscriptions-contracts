# Fuzzing the subscription contract

Coverage-guided fuzzing with [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz).
The target `entry_points` drives the contract's entry points with arbitrary
arguments and checks that nothing panics outside a contract error and that
remaining cap never goes negative.

## Covered operations

The fuzz `Op` enum exercises the following paths:

- `Subscribe` — create a subscription with arbitrary amount, interval and
  expiry.
- `Charge` — charge a subscription and advance the ledger.
- `Cancel` — cancel an active subscription.
- `Pause` — pause an active subscription.
- `Resume` — resume a paused subscription.
- `Advance` — advance the ledger timestamp without charging.
- `BatchCharge` — charge several subscriptions in a single call, including
  mixed active/paused/cancelled inputs and empty batches.
- `SubscribeUntil` — create a subscription with an expiry (`subscribe_until`)
  and drive it past that expiry to exercise the expiry/termination path.
- `SubscribeWithTrial` — create a subscription that starts with a trial
  period, then charge across the trial boundary to exercise the trial-period
  path.

## Running

```bash
cargo install cargo-fuzz --locked
rustup toolchain install nightly
cd fuzz
cargo +nightly fuzz run entry_points -- -max_total_time=60
```

For a longer soak (e.g. the 1-hour run described below):

```bash
cargo +nightly fuzz run entry_points -- -max_total_time=3600
```

A crash is saved under `fuzz/artifacts/entry_points/`; reproduce it with
`cargo +nightly fuzz run entry_points <artifact-path>`. A scheduled workflow
(`.github/workflows/fuzz.yml`) runs this weekly for a fixed time and uploads
any crash as a build artifact.

## 1-hour local run (issue #165)

Command:

```bash
cargo +nightly fuzz run entry_points -- -max_total_time=3600
```

Results:

- Ran for the full 3600 seconds without terminating early.
- No crashes, panics, or invariant violations were observed. In particular,
  remaining cap never went negative and no non-contract-error panic occurred
  across the `batch_charge`, `subscribe_until`, and trial-subscription paths.
- No artifacts were written to `fuzz/artifacts/entry_points/`.

If a future run does surface a crash, reproduce it with:

```bash
cargo +nightly fuzz run entry_points fuzz/artifacts/entry_points/<artifact>
```

and attach the artifact to the issue.