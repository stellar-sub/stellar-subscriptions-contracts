# Audit-readiness checklist

This document collects everything a third-party auditor needs to scope the
engage­ment, reproduce the build, and understand the security model.

## Scope

| Item | Detail |
|---|---|
| Repository | `https://github.com/stellar-sub/stellar-subscriptions-contracts` |
| Commit under review | `a746709` (see [DEPLOYMENTS.md](../DEPLOYMENTS.md)) |
| Contracts in scope | `contracts/subscription`, `contracts/plan`, `contracts/registry` |
| Language / runtime | Rust, Soroban (Stellar smart-contract SDK) |
| Token standard | [SEP-0041](https://github.com/stellar/stellar-protocol/blob/master/ecosystem/sep-0041.md) fungible token |
| Out of scope | Front-end, API server, off-chain indexer |

## Build instructions

### Prerequisites

- Rust (version pinned in `rust-toolchain.toml`; installed automatically by
  `rustup` on first build)
- `wasm32v1-none` target, `rustfmt`, `clippy` — added by `rust-toolchain.toml`
- Stellar CLI 26.1.0+ for the wasm build and deployment:
  ```bash
  cargo install --version 26.1.0 stellar-cli --locked
  ```

### Reproduce the build

```bash
git clone https://github.com/stellar-sub/stellar-subscriptions-contracts
cd stellar-subscriptions-contracts
git checkout a746709   # pinned commit
make wasm              # cargo build --workspace --release --target wasm32v1-none
```

The wasm artefacts land in `target/wasm32v1-none/release/`.
Compare their sha256 against the hashes in [DEPLOYMENTS.md](../DEPLOYMENTS.md)
to confirm the deployed code matches the repository.

## Running the test suite

```bash
make test              # cargo test --workspace
make check             # test + clippy + fmt-check + wasm build (mirrors CI)
```

To fuzz deeper than the 48-case default:

```bash
PROPTEST_CASES=500 cargo test -p subscription fuzz
```

To prove the invariants against the live Testnet deployment:

```bash
make verify            # runs scripts/verify-testnet.sh
```

## Deployed contract ids (Testnet)

See [DEPLOYMENTS.md](../DEPLOYMENTS.md) for the current contract ids, wasm
hashes, and the results of the on-chain invariant verification run.

| Contract | Contract id |
|---|---|
| Subscription | `CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM` |
| Plan | `CDKFRKPA2EIYMW2GX2FY4ZFQ2VAFS5UZD5BHAZLCIGRA7N24N2DFAYN3` |
| Registry | `CB7HBXFNBLUSBY3SMIYUPVTE3JZTISYRMZB7XXPUC3ZPLIDNPTKUX645` |

Mainnet is not yet deployed.

## Security model and invariants

The subscription contract enforces three invariants before any funds move.
A full description of each, and the tests that prove them, is in
[docs/subscription.md](subscription.md#the-three-invariants).

| Invariant | One-line rule | Test file |
|---|---|---|
| **CAP** | `total_charged` never exceeds `total_cap` | `contracts/subscription/src/test/invariant_cap.rs` |
| **INTERVAL** | At most one charge per billing interval | `contracts/subscription/src/test/invariant_interval.rs` |
| **REVOCATION** | After the subscriber cancels, no charge ever succeeds | `contracts/subscription/src/test/invariant_revocation.rs` |

`contracts/subscription/src/test/fuzz.rs` drives random sequences of charge /
pause / resume / cancel and asserts all three after every step.

## Auth model summary

| Actor | Can do | Cannot do |
|---|---|---|
| Subscriber | subscribe, pause, resume, cancel, refresh_allowance | charge |
| Merchant | charge | cancel, pause, resume, anything subscriber-only |
| Admin | initialize, set_plan_contract, set_registry (each once) | touch any subscription or plan |

See [docs/subscription.md](subscription.md#who-may-call-what) for the full
function-level table.

## Known limits and open questions

### Known limits

- `get_by_subscriber`, `get_by_merchant`, `get_active_subscriptions` and the
  allowance recomputation walk every subscription ever created for a given
  party. Parties with very large subscription counts should use an off-chain
  indexer reading the events. See the caveats section in
  [docs/registry.md](registry.md#caveats) and
  [docs/subscription.md](subscription.md#trust-assumptions-and-limits).
- `total_charged_volume` in the registry sums raw token units across **all
  tokens**. It is an activity signal, not a currency amount.
- Mainnet is not yet deployed; the live deployment is Testnet only.
- Linked contracts (plan, registry) affect liveness but not safety: a broken
  registry cannot make `cancel` fail and cannot make a charge succeed that the
  invariants reject.

### Open questions

- **Upgrade path.** There is no on-chain upgrade mechanism. Re-deployment
  requires re-initialization and re-linking before any subscriptions are
  created. Whether to add a controlled upgrade path is undecided.
- **Token allowance expiry.** Allowances are approved until the network's
  maximum entry lifetime (~120 days). Subscribers must call
  `refresh_allowance` to keep long-running subscriptions chargeable. Whether
  to surface an off-chain reminder is an open product question.
- **Mainnet deployment.** No governance process for mainnet deployment exists
  yet; the admin key is a single account.
- **Per-token volume.** The registry's `total_charged_volume` mixes tokens.
  A per-token breakdown belongs in the API layer reading events.

## Reference docs

- [docs/subscription.md](subscription.md) — full function reference, charge
  order of operations, error codes, trust assumptions
- [docs/plan.md](plan.md) — plan lifecycle, events, error codes
- [docs/registry.md](registry.md) — registry data model, update rules,
  caveats
- [DEPLOYMENTS.md](../DEPLOYMENTS.md) — live contract ids, wasm hashes,
  on-chain invariant proof results
- [CONTRIBUTING.md](../CONTRIBUTING.md) — how to add features without
  weakening the invariants
- [SECURITY.md](../SECURITY.md) — vulnerability disclosure policy
