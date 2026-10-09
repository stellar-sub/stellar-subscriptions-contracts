# Changelog

Notable changes to the contracts. Contract error codes and status
discriminants are append-only and never renumbered.

## Unreleased

### Added
- `scripts/verify-testnet.sh` and `make verify`: prove CAP, INTERVAL and
  REVOCATION against a live deployment. 20 of 20 checks pass on Testnet;
  see [DEPLOYMENTS.md](DEPLOYMENTS.md).
- [SECURITY.md](SECURITY.md): guarantees, trust assumptions, known limits and
  how to report a problem.
- Tests: a subscriber can stop charges by zeroing the token allowance, an
  allowance below one period blocks charging until restored, the allowance
  caps charges even when the cap would allow more, and a token that panics
  cannot stop a subscriber from cancelling.
- CI runs the invariant fuzz test 500 cases deep; `PROPTEST_CASES` sets the
  depth locally (default 48).

## 0.1.0 (Testnet, 2026-09-15)

### Added
- **Subscription** contract: `subscribe`, `charge`, `cancel`, `pause`,
  `resume`, `refresh_allowance`, views, and events for every state change.
  CAP, INTERVAL and REVOCATION enforced in `charge()` with checked arithmetic.
- **Plan** contract: merchant billing plans with deactivation.
- **Registry** contract: plan and subscription index with aggregate stats,
  writable only by the linked contracts.
- `scripts/deploy.sh`: deploy, initialize and link all three, with an
  optional live demo.

### Fixed
- Allowance expiry is aligned to a ledger window so the `approve` a wallet
  signs during simulation still matches when the transaction lands. Before
  this, `subscribe` passed simulation and then failed on submission.
