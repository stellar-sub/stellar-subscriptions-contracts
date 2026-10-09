# Security

These contracts move recurring payments, so the security model is the product.
This document says what is guaranteed, what is trusted, what is **not** proven,
and how to report a problem.

## What is guaranteed

Three invariants hold for every subscription, always:

1. **CAP.** The sum of everything charged never exceeds the `total_cap` the
   subscriber authorized.
2. **INTERVAL.** At most one charge per billing interval, even from the right
   merchant, and a late charge does not unlock a burst of catch-up charges.
3. **REVOCATION.** After the subscriber cancels, no charge succeeds, ever.

Evidence for each, in increasing strength:

- **Unit tests per invariant**, written before the implementation:
  `contracts/subscription/src/test/invariant_cap.rs`, `invariant_interval.rs`,
  `invariant_revocation.rs`.
- **Property-based fuzzing** (`test/fuzz.rs`): random sequences of charge,
  pause, resume and cancel with random time gaps, checking all three after
  every step. Runs 48 cases locally and 500 in CI; a one-off run of 400 found
  no counterexample.
- **Live checks on Testnet** (`scripts/verify-testnet.sh`): the contracts
  refuse each violation with the expected error code. Results and transaction
  links are in [DEPLOYMENTS.md](DEPLOYMENTS.md).

The cap, interval and cancellation are enforced by the subscription contract
in `charge()` before any funds move. Nothing in the web app or the other two
contracts is needed for them to hold.

## Who can do what

| Actor | Can | Cannot |
|---|---|---|
| Subscriber | subscribe, pause, resume, cancel, refresh their allowance | be charged more than their cap, or after cancelling |
| Merchant | charge their own subscriptions, within the interval and cap | charge anyone else's, cancel for a subscriber, charge after cancel |
| Admin | link the plan and registry contracts, once, before first use | touch any subscription, change a link once set, charge, cancel, pause |
| Anyone | read state | write anything |

A subscriber can also stop charges without cancelling by lowering their token
allowance directly on the token. The charge then fails with `TransferFailed`
and changes nothing (tested).

## What is trusted

- **The token.** Chosen by the subscriber and merchant, not by this contract.
  A token that refuses a transfer makes the charge fail cleanly with no state
  change. A token that **panics** when asked to approve cannot stop a
  subscriber from cancelling; the failure is reported in the `cancelled`
  event as `allowance_updated: false` (tested in `test/hostile_token.rs`).
  Pick tokens you trust: a malicious token can still misreport balances.
- **The linked registry.** It can make `subscribe`, `charge`, `pause` and
  `resume` fail if it breaks, which is a liveness problem and never a safety
  one: it cannot make a charge succeed that the invariants refuse, and it
  cannot block `cancel`.
- **The Soroban host and the Stellar Asset Contract**, as for any contract on
  the network.

## Known limits and what is not proven

- **Resource exhaustion in a hostile token is untested.** `cancel` calls the
  token with a fallible call, so errors and panics are contained. A token that
  deliberately burns the whole transaction budget cannot be simulated in the
  native test environment, and we have not shown that `cancel` survives it.
  Subscribers can avoid the question by using well-known tokens (such as the
  native asset). Treat this as an open item for an audit.
- **Large histories cost more.** `get_by_subscriber`, `get_by_merchant`, and
  the allowance recomputation in `subscribe`, `cancel` and `refresh_allowance`
  walk every subscription that party has ever had. A party with a very large
  number of subscriptions can hit the per-transaction resource limit. Use an
  indexer on the emitted events for those.
- **Allowances expire.** The contract approves until the network's maximum
  entry lifetime (aligned to a ~1-day window so signatures stay valid between
  simulation and submission). Past that, charges fail with `TransferFailed`
  until the subscriber calls `refresh_allowance`. Nothing is lost; it fails
  closed.
- **Partial cap.** A cap that is not a whole number of periods leaves a
  remainder that can never be charged, by design.
- **Amounts across tokens.** The registry's `total_charged_volume` adds raw
  units across all tokens. It is an activity measure, not a value.
- **Not audited.** No third-party audit has been done. These contracts are
  deployed on Testnet only.

## Reporting a vulnerability

Please do **not** open a public issue for a security problem. Use GitHub's
private vulnerability reporting on this repository (Security tab, "Report a
vulnerability"). Include the contract, the function, and the smallest
sequence of calls that shows the problem. A failing test is ideal.

We treat any way to violate CAP, INTERVAL or REVOCATION as critical.
