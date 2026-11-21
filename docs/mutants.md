# Mutation testing (cargo-mutants)

Mutation testing changes the code (for example `>` to `>=`, or a return value
to a default) and checks that some test fails. A mutant that no test catches
is a **surviving mutant**: a behaviour the tests do not pin down.

## Ongoing check

`.github/workflows/mutants.yml` runs `cargo mutants --workspace --timeout 300`
every Monday and on manual dispatch. The job fails when a mutant survives, and
uploads `mutants.out` as an artifact. The weekly schedule catches code added
after the last run (issue #80 only covered the code that existed then).

## Accepted surviving mutants

A surviving mutant is either fixed with a test, or recorded here as an
accepted risk and excluded with `exclude_re` in `.cargo/mutants.toml`. Each
entry needs a reason.

| Mutant (file:function) | Why it is accepted | Tracking |
|------------------------|--------------------|----------|

_No accepted risks yet. Fill this in from the first baseline run._

## Baseline

Baseline scan on the current codebase: **not yet run.** The first scheduled or
manual run of `mutants.yml` produces the baseline. Record its totals here
(caught / missed / unviable / timeout) and the date it was run.

## Running locally

```sh
cargo install cargo-mutants --locked
cargo mutants --file contracts/subscription/src/lib.rs
```

See also [CONTRIBUTING.md](../CONTRIBUTING.md).
