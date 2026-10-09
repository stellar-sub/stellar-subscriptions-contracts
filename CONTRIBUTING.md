# Contributing

Thanks for helping. This repository holds money-moving contracts, so the bar
for changes to the subscription contract is high. Please read
[The invariants](#the-invariants) before opening a PR.

## Local setup

1. **Rust.** Install [rustup](https://rustup.rs). The pinned toolchain,
   the `wasm32v1-none` target, `rustfmt` and `clippy` are installed
   automatically from `rust-toolchain.toml` the first time you run `cargo`.
2. **Stellar CLI.** Needed to build release wasm and to deploy. Follow
   <https://developers.stellar.org/docs/tools/cli/install-cli>, then check:

   ```bash
   stellar --version
   ```

3. **Clone and test.**

   ```bash
   git clone https://github.com/stellar-sub/stellar-subscriptions-contracts
   cd stellar-subscriptions-contracts
   make test
   ```

## Build, test, deploy

| Task | Command |
|---|---|
| Run all tests | `make test` (`cargo test --workspace`) |
| Run one contract's tests | `cargo test -p subscription` (or `plan`, `registry`) |
| Run only the invariant tests | `cargo test -p subscription invariant_` |
| Lint | `make clippy` |
| Format | `make fmt` |
| Build wasm (CLI) | `make build` |
| Build wasm (cargo only, as CI does) | `make wasm` |
| Everything CI checks | `make check` |
| Deploy to Testnet | `make deploy` |

`make deploy` runs `scripts/deploy.sh`, which creates and funds a
`subs-deployer` identity if needed, deploys the plan, registry and
subscription contracts, initializes them and links them. Set
`NETWORK` / `IDENTITY` to override, and `DEMO=1` to also run a live
plan → subscribe → charge that proves a same-interval second charge is
rejected on-chain. Contract ids are written to `.deployments/<network>.env`;
copy them into `DEPLOYMENTS.md`.

Linking is one-shot: each contract accepts its links once and only before
its first plan or subscription. To re-link, deploy fresh instances.

## The invariants

A subscription is a promise from the subscriber: "you may take *this much*,
*this often*, *until I say stop*." Each part of that sentence is an invariant.

### CAP — never more than authorized

The total charged to a subscription can never exceed the `total_cap` the
subscriber agreed to. A charge that would go over is refused outright, not
partially filled.

**Why it matters:** the cap is the subscriber's worst-case loss. If a merchant
is compromised, buggy, or malicious, the cap is the most they can take. If CAP
can be broken, a subscriber has effectively handed over their whole balance.

### INTERVAL — never more often than authorized

At most one charge per billing interval. A second charge before
`next_charge_ledger` is refused even when it comes from the correct merchant,
and a merchant who charges late cannot then charge several times in a row to
catch up.

**Why it matters:** without it, a merchant could drain the entire cap in one
ledger. CAP bounds *how much*; INTERVAL bounds *how fast*, giving the
subscriber time to notice and cancel.

### REVOCATION — never after the subscriber says stop

Once the subscriber cancels, no charge succeeds again, ever. Cancelling is
final, only the subscriber can do it, and nothing — not the merchant, the
admin, the token or the registry — can block or undo it.

**Why it matters:** consent that cannot be withdrawn is not consent. This is
the subscriber's escape hatch when anything else goes wrong.

All three are enforced in `charge()` **before** any funds move, and each has
its own test file: `test/invariant_cap.rs`, `test/invariant_interval.rs`,
`test/invariant_revocation.rs`. `test/fuzz.rs` checks all three after every
step of random action sequences.

## Adding a feature without weakening the invariants

Work through this checklist for any change to `contracts/subscription`, and
say in the PR description how each point is satisfied.

1. **Write the invariant-facing tests first.** If your feature touches
   charging, status or amounts, add tests showing CAP, INTERVAL and REVOCATION
   still hold with it — ideally failing before your implementation exists.
   Add the feature's actions to `test/fuzz.rs` so they get interleaved with
   everything else.
2. **Never add a path that moves funds outside `charge()`.** `charge()` is the
   single place the three checks run. A second transfer path is a second
   place they must all be re-implemented correctly.
3. **Never add a way out of a final state.** Nothing may move a subscription
   out of `Cancelled` or `Exhausted`.
4. **Never let anything block `cancel`.** Follow-up work in `cancel` (allowance,
   registry, anything new) must be best-effort and report its outcome in the
   event rather than failing the call.
5. **Keep the auth split.** `charge` requires the merchant's signature;
   `cancel`, `pause`, `resume` and anything else that changes the
   subscriber's consent require the subscriber's. Admin functions must not
   touch subscriptions. Add a `mock_auths` test proving the wrong party's
   signature is rejected.
6. **Checked arithmetic, compared before transfer.** Any arithmetic on
   `total_charged`, caps or ledgers uses `checked_*` and returns
   `Error::Overflow`. Compare against `total_cap` before any transfer.
7. **No panics.** Return an `Error` variant; never `unwrap`, `expect`,
   `panic!` or `todo!` in contract code. Error codes are append-only.
8. **Emit an event** for any new state change.
9. **Keep signed nested calls stable across ledgers.** Wallets sign the
   authorization tree they saw when simulating, and the transaction lands a
   few ledgers later. Any argument of a nested call the subscriber signs
   (such as the token `approve`) must not be derived from the exact current
   ledger, or the call will pass simulation and fail on-chain. Add a case to
   `test/simulation_drift.rs`.

### Worked example: `refresh_allowance`

Token allowances expire, so long subscriptions need a way to renew theirs.
`refresh_allowance(subscriber, token)` was added like this:

- **Auth:** it changes what the subscriber has approved, so it requires the
  **subscriber's** signature (point 5).
- **No new fund path:** it only calls `approve`; funds still move only in
  `charge()` (point 2).
- **Cannot loosen CAP:** the approved amount is recomputed from contract state
  as the sum of remaining caps of the subscriber's non-final subscriptions.
  Even an allowance larger than that could not be spent past a subscription's
  cap, because `charge()` checks the cap itself (point 6).
- **Tests first:** a test zeroes the allowance, shows `charge` failing with
  `TransferFailed` and leaving state untouched, then shows `refresh_allowance`
  restoring charging on the normal schedule — and that a cancelled
  subscription contributes nothing to the refreshed amount (point 1).

## Code style

- `cargo fmt` and `cargo clippy -- -D warnings` must pass (CI enforces both).
- Match the surrounding code: module layout (`types`, `errors`, `storage`,
  `lib`), doc comments on public functions explaining *why*, and test names
  that read as sentences.
- Commits: [Conventional Commits](https://www.conventionalcommits.org)
  (`feat(subscription): ...`, `test(plan): ...`, `docs: ...`). One logical
  change per commit.

## Pull requests

- Keep PRs focused. Invariant-affecting changes should not be mixed with
  refactors.
- CI must be green: tests, clippy, fmt and the wasm build.
- Changes to `contracts/subscription` need a review that explicitly checks
  the checklist above.

## Sister repositories

- Web app: https://github.com/stellar-sub/stellar-subscriptions-web
- API + Docs: https://github.com/stellar-sub/stellar-subscriptions-api-docs
