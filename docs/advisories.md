# Handling a dependency advisory

CI checks dependencies two ways: `cargo audit` (`.github/workflows/audit.yml`)
and `cargo deny check` (`.github/workflows/deny.yml`, configured in
`deny.toml`). Both read the RustSec advisory database. `cargo deny` also
checks licences, banned crates and registry sources.

Run it locally with `cargo install cargo-deny --locked && cargo deny check`.

## When an advisory fails the build

1. **Read it.** The output links the RustSec entry (`RUSTSEC-YYYY-NNNN`).
   Find out whether the vulnerable code is reachable from the contracts.
2. **Prefer upgrading.** Run `cargo update -p <crate>` or bump the
   `soroban-sdk` version in the workspace `Cargo.toml`, then run `make check`.
3. **If no fixed version exists**, and the affected code is not reachable
   from the contracts, add a reviewed exception to `ignore` in `deny.toml`
   with a comment giving the advisory id, the reason, and a date or condition
   to revisit. Open an issue to track removing it.
4. **Never ignore silently.** An exception without a reason is rejected in
   review. Exceptions in `deny.toml` must not be used to hide a dependency
   that a contract path actually reaches.

## Unmaintained and yanked crates

These fail the deny job. Treat them like advisories: upgrade or replace if
possible, otherwise add a documented exception.

## New licences

If a new dependency brings a licence not in `[licenses] allow`, check it is
compatible with this repository before adding it to the list.
