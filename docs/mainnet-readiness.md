# Mainnet Readiness Checklist

> **Status: NOT DEPLOYED.**
>
> The Stellar Subscriptions contracts are **not deployed to mainnet**. No mainnet
> deployment exists, and none should be attempted until the audit and the
> readiness steps below are complete. This document is the single place where
> the steps and blockers for a mainnet launch are written down.

This checklist covers what must happen before, during and after a mainnet
deployment. Owners are left as `TBD` placeholders for maintainers to fill in.
See [`DEPLOYMENTS.md`](../DEPLOYMENTS.md) for the record of existing
deployments and [`SECURITY.md`](../SECURITY.md) for the security policy and
vulnerability reporting process.

## Summary of blockers

Mainnet stays undeployed until every blocker below is closed.

| # | Blocker | Owner | Status |
|---|---------|-------|--------|
| 1 | Independent security audit completed and findings resolved | TBD | Not started |
| 2 | Key management plan in place (deployer, admin, upgrade keys) | TBD | Not started |
| 3 | Monitoring and alerting live on testnet | TBD | Not started |
| 4 | Incident response plan reviewed and exercised | TBD | Not started |
| 5 | Deployment record template and rollback plan agreed | TBD | Not started |
| 6 | Communication plan and launch announcement approved | TBD | Not started |

## 1. Audit

- [ ] Scope the audit (contracts, build/reproducibility, deployment scripts) — Owner: TBD
- [ ] Select an independent auditor — Owner: TBD
- [ ] Complete the audit and receive the report — Owner: TBD
- [ ] Triage every finding and fix or explicitly accept each one — Owner: TBD
- [ ] Re-audit or verify fixes with the auditor — Owner: TBD
- [ ] Publish the audit report and a summary of resolutions — Owner: TBD
- [ ] Confirm `make check` passes on the audited commit — Owner: TBD
- [ ] Confirm CAP, INTERVAL and REVOCATION behaviour is unchanged and error codes are append-only — Owner: TBD

## 2. Key management

- [ ] Define roles and keys: deployer, admin, upgrade/operator, treasury — Owner: TBD
- [ ] Decide custody model (hardware wallet, multisig, threshold) — Owner: TBD
- [ ] Document key generation, storage and backup procedures — Owner: TBD
- [ ] Document key rotation and revocation procedures — Owner: TBD
- [ ] Confirm admin/upgrade authority is a multisig or equivalent — Owner: TBD
- [ ] Record public keys and their roles in the deployment record — Owner: TBD
- [ ] Run a dry-run of the key ceremony on testnet — Owner: TBD

## 3. Monitoring

- [ ] Define the metrics that matter (contract invocations, failures, balances, events) — Owner: TBD
- [ ] Set up dashboards for contract and account activity — Owner: TBD
- [ ] Configure alerts with thresholds and on-call routing — Owner: TBD
- [ ] Monitor contract balances and low-balance conditions — Owner: TBD
- [ ] Monitor for unexpected admin/upgrade operations — Owner: TBD
- [ ] Validate monitoring and alerts on testnet before mainnet — Owner: TBD

## 4. Incident response plan

- [ ] Write the incident response plan (severity levels, roles, escalation) — Owner: TBD
- [ ] Define pause/upgrade/rollback levers and who can trigger them — Owner: TBD
- [ ] Define the process for coordinated disclosure per [`SECURITY.md`](../SECURITY.md) — Owner: TBD
- [ ] Establish an incident communication channel — Owner: TBD
- [ ] Run a tabletop exercise of the plan — Owner: TBD
- [ ] Review and sign off on the plan — Owner: TBD

## 5. Deployment record

- [ ] Agree the deployment record format and location in [`DEPLOYMENTS.md`](../DEPLOYMENTS.md) — Owner: TBD
- [ ] Record the exact commit, build and wasm hash to be deployed — Owner: TBD
- [ ] Verify a reproducible build matches the audited artifact — Owner: TBD
- [ ] Document the step-by-step deployment procedure — Owner: TBD
- [ ] Document the rollback procedure — Owner: TBD
- [ ] Record contract IDs, network passphrase and admin keys after deploy — Owner: TBD
- [ ] Have a second maintainer verify the record — Owner: TBD

## 6. Communication plan

- [ ] Draft the launch announcement and pre-launch notice — Owner: TBD
- [ ] Define channels (repo, social, community) and who posts — Owner: TBD
- [ ] Prepare user-facing docs and migration notes — Owner: TBD
- [ ] Prepare a post-deployment status update — Owner: TBD
- [ ] Prepare holding statements for incident scenarios — Owner: TBD
- [ ] Get final approval before any public mainnet announcement — Owner: TBD

## Sign-off

Mainnet deployment is approved only when every blocker above is closed and
signed off by the maintainers.

| Role | Name | Date |
|------|------|------|
| Maintainer | TBD | TBD |
| Maintainer | TBD | TBD |
| Security reviewer | TBD | TBD |