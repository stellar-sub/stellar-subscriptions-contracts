---
name: Bug report
about: Report a defect in the contracts, tooling or docs
title: ""
labels: bug
---

<!--
Before you start, please read CONTRIBUTING.md:
https://github.com/stellar-sub/stellar-subscriptions-contracts/blob/main/CONTRIBUTING.md

Contract changes need tests and must not weaken CAP, INTERVAL or REVOCATION;
error codes are append-only. `make check` must pass.
-->

## Context

<!-- What were you doing? Which contract, function, tool or doc is affected? Include versions/commits. -->

## Expected behaviour

<!-- What did you expect to happen? -->

## Actual behaviour

<!-- What actually happened? Include error codes, logs or transaction hashes where relevant. -->

## Reproduction steps

1.
2.
3.

## Invariant checklist

<!-- Confirm the bug does not require weakening any invariant. Tick the boxes that apply. -->

- [ ] I have read CONTRIBUTING.md
- [ ] This report does not require weakening the CAP invariant
- [ ] This report does not require weakening the INTERVAL invariant
- [ ] This report does not require weakening the REVOCATION invariant
- [ ] Any fix will include tests and pass `make check`
- [ ] Any new error codes will be appended only (no reordering or reuse)