# Testing

`contracts/subscription/src/test/` holds that contract's whole test suite:
unit, invariant, fuzz, integration and drift tests side by side. This is
where each one lives and what it is for.

## The shared `Setup` helper

`test/setup.rs` is not a test file itself (`mod.rs` does not list it as a
suite, but every suite uses it). `Setup::new()` builds one ready-to-use
world:

- a fresh `Env` with every auth check mocked (`env.mock_all_auths()`);
- the ledger sequence set to `START_LEDGER`;
- an admin, subscriber and merchant address;
- a real Stellar asset contract used as the token, with the subscriber
  pre-minted `START_BALANCE`;
- the subscription contract deployed and initialized with that admin.

From there, helpers on `Setup` cover what almost every test needs:
`subscribe()` / `subscribe_with(...)` to open a subscription with the
default or custom terms, `advance(ledgers)` / `set_ledger(n)` to move time,
`balance`, `allowance` and `sub` to read state, and `snapshot(id)` to capture
everything a charge could touch (`Snapshot`) in one call so a test can assert
nothing changed after a rejected call. `expect_err` asserts a `try_*` client
call failed with exactly the expected `Error`, distinguishing a contract
error from a host error or an unexpected success.

## What each test file covers

| File | Covers |
|---|---|
| `invariant_cap.rs` | **CAP** — a charge is never allowed to push `total_charged` past `total_cap`. |
| `invariant_interval.rs` | **INTERVAL** — a charge before `next_charge_ledger` is rejected, even from the right merchant. |
| `invariant_revocation.rs` | **REVOCATION** — once cancelled, no charge on that subscription ever succeeds again. |
| `fuzz.rs` | Randomised sequences of charge / pause / resume / cancel with random ledger gaps; all three invariants must hold after every step. |
| `auth.rs` | Every state-changing function requires the correct party's signature and fails with no auth at all. |
| `charge.rs` | The happy path for `charge`, and how pause / resume move the charge schedule. |
| `allowance.rs` | The token allowance that backs each subscriber's open subscriptions. |
| `hostile_token.rs` | REVOCATION still holds when the underlying token misbehaves on cancel's follow-up calls. |
| `simulation_drift.rs` | `subscribe`, `cancel` and `refresh_allowance` signed several ledgers before they land still succeed, because the nested `approve`'s arguments are stable across that drift. |
| `expiry.rs` | `subscribe_until`: an optional end ledger after which a subscription can never be charged. |
| `events.rs` | Each state-changing call emits exactly the event it should, with token events filtered out. |
| `validation.rs` | Initialization and `subscribe` parameter checks, plus lifecycle misuse (double-pause, etc). |
| `views.rs` | `remaining_cap`, `is_chargeable` and the per-party listing views. |
| `merchant_isolation.rs` | Charging one merchant's subscription never touches another merchant's. |
| `mock_registry.rs` | A drop-in registry double that can be armed to fail, used by other suites. |
| `integration.rs` | All three contracts deployed and linked together, checked against real plans and a real registry. |

## Adding a test: a worked example

Say you are adding a test that cancelling a subscription twice fails the
second time. It is a lifecycle-misuse case, so it belongs in
`validation.rs`.

1. Open `contracts/subscription/src/test/validation.rs`. It already imports
   `super::setup::{expect_err, Setup, ...}` and `crate::{Error, ...}`, which
   is everything this test needs.
2. Add the test using `Setup`:

   ```rust
   #[test]
   fn cancel_twice_fails_the_second_time() {
       let s = Setup::new();
       let id = s.subscribe();

       s.client.cancel(&s.subscriber, &id);

       let result = s.client.try_cancel(&s.subscriber, &id);
       expect_err(result, Error::SubscriptionCancelled);
   }
   ```

3. No new entry is needed in `test/mod.rs` — `validation` is already
   declared there as `mod validation;`.
4. Run just this contract's tests: `cargo test -p subscription`, or narrow
   further with `cargo test -p subscription cancel_twice`.

The same pattern applies in `contracts/plan/src/test.rs` and
`contracts/registry/src/test.rs`, which are single files rather than a
`test/` directory, since those contracts are smaller.
