# Error Code Reference

Every error that can be returned by the Subscription, Plan, and Registry
contracts is listed here. Each contract has its own numeric space — code `4`
in the Subscription contract is not the same as code `4` in the Plan contract.
Use the per-contract tables for precise mapping, or the [combined quick-lookup
table](#combined-quick-lookup) at the bottom to scan all codes at once.

---

## Subscription Contract

_Path: `contracts/subscription/src/errors.rs`_

| Code | Name | When it fires | What to tell the user |
|-----:|------|---------------|-----------------------|
| 1 | `AlreadyInitialized` | `initialize` called on a contract that is already set up | "This subscription contract has already been initialized — no action needed." |
| 2 | `NotInitialized` | Any call made before `initialize` has been run | "The subscription contract has not been initialized yet. Contact the operator." |
| 3 | `Unauthorized` | Caller is not the contract admin | "Only the contract admin can perform this action." |
| 4 | `InvalidAmount` | `amount_per_period` is zero or negative | "The charge amount must be greater than zero." |
| 5 | `InvalidInterval` | `interval_ledgers` is zero | "The billing interval must be at least one ledger." |
| 6 | `InvalidCap` | `total_cap` is less than `amount_per_period` | "The spending cap must be at least as large as one period's charge amount." |
| 7 | `SameParty` | Subscriber address equals merchant address | "The subscriber and merchant cannot be the same account." |
| 8 | `SubscriptionNotFound` | No subscription exists for the given ID | "Subscription not found. Check the subscription ID and try again." |
| 9 | `NotMerchant` | The account calling `charge` is not the subscription's registered merchant | "Only the merchant who created this subscription can charge it." |
| 10 | `NotSubscriber` | A subscriber-only action (pause, resume, cancel) was called by someone else | "Only the subscriber can perform this action." |
| 11 | `IntervalNotElapsed` | A charge was attempted before the next billing interval has started | "It is too early to charge — the current billing period has not ended yet." |
| 12 | `CapExceeded` | Charging would push the total charged over the authorized `total_cap` | "This charge would exceed the subscriber's authorized spending cap." |
| 13 | `Overflow` | Checked arithmetic overflowed (e.g. `total_charged + amount` wraps u128) | "An arithmetic overflow occurred. Please report this as a bug." |
| 14 | `SubscriptionCancelled` | Any action on a subscription that has been cancelled | "This subscription has been cancelled and can no longer be used." |
| 15 | `SubscriptionPaused` | A charge was attempted while the subscription is paused | "This subscription is paused. The subscriber must resume it before it can be charged." |
| 16 | `SubscriptionExhausted` | The subscription's `total_cap` has been fully consumed | "This subscription has reached its spending cap and is now exhausted." |
| 17 | `AlreadyPaused` | `pause` called on a subscription that is already paused | "This subscription is already paused." |
| 18 | `NotPaused` | `resume` called on a subscription that is not paused | "This subscription is not paused, so it cannot be resumed." |
| 19 | `TransferFailed` | The token contract refused the `transfer_from` call; no state was changed | "Payment failed — the token transfer was rejected. Check the subscriber's balance and allowance." |
| 20 | `ApprovalFailed` | The token contract refused to set the required allowance | "Failed to set token allowance. The subscriber may need to approve the contract manually." |
| 21 | `PlanNotFound` | A `plan_id` was given but the Plan contract has no matching plan | "The referenced billing plan does not exist." |
| 22 | `PlanInactive` | The referenced plan exists but has been deactivated | "The referenced billing plan is no longer active." |
| 23 | `PlanMismatch` | The subscription's stored terms differ from the current plan terms | "The subscription terms do not match the plan. The plan may have changed since this subscription was created." |
| 24 | `AlreadyConfigured` | A one-time link (plan/registry contract) is already set, or the subscription ID already exists | "This resource has already been configured and cannot be changed." |
| 25 | `PlanContractNotSet` | A non-zero `plan_id` was supplied but no Plan contract address is linked | "A billing plan was specified, but no Plan contract is linked to this Subscription contract. Contact the operator." |
| 26 | `RegistryUpdateFailed` | The Registry contract rejected a status/stats update | "Failed to sync with the Registry. Please report this as a bug." |
| 27 | `InvalidExpiry` | The `end_ledger` supplied is not strictly after the current ledger | "The expiry ledger must be in the future." |
| 28 | `SubscriptionExpired` | The current ledger is at or past the subscription's `end_ledger` | "This subscription has expired." |

---

## Plan Contract

_Path: `contracts/plan/src/errors.rs`_

| Code | Name | When it fires | What to tell the user |
|-----:|------|---------------|-----------------------|
| 1 | `AlreadyInitialized` | `initialize` called on an already-initialized contract | "This Plan contract has already been initialized." |
| 2 | `NotInitialized` | Any call before `initialize` | "The Plan contract has not been initialized yet. Contact the operator." |
| 3 | `Unauthorized` | Caller is not the contract admin | "Only the contract admin can perform this action." |
| 4 | `InvalidName` | Plan name is empty or exceeds 64 bytes | "Plan name must be between 1 and 64 bytes." |
| 5 | `InvalidAmount` | `amount_per_period` is zero or negative | "The charge amount must be greater than zero." |
| 6 | `InvalidInterval` | `interval_ledgers` is zero | "The billing interval must be at least one ledger." |
| 7 | `InvalidCap` | `default_cap` is less than `amount_per_period` | "The default spending cap must be at least as large as one period's charge amount." |
| 8 | `PlanNotFound` | No plan exists for the given plan ID | "Plan not found. Check the plan ID and try again." |
| 9 | `NotPlanMerchant` | A deactivation call was made by someone other than the plan's creating merchant | "Only the merchant who created this plan can deactivate it." |
| 10 | `PlanAlreadyInactive` | `deactivate` called on a plan that is already inactive | "This plan is already inactive." |
| 11 | `Overflow` | Checked arithmetic overflowed | "An arithmetic overflow occurred. Please report this as a bug." |
| 12 | `AlreadyConfigured` | The Registry contract is already linked, or the plan ID already exists | "This resource has already been configured and cannot be changed." |
| 13 | `RegistryUpdateFailed` | The linked Registry contract refused the plan-count update | "Failed to register the plan with the Registry. Please report this as a bug." |

---

## Registry Contract

_Path: `contracts/registry/src/errors.rs`_

| Code | Name | When it fires | What to tell the user |
|-----:|------|---------------|-----------------------|
| 1 | `AlreadyInitialized` | `initialize` called on an already-initialized contract | "This Registry contract has already been initialized." |
| 2 | `NotInitialized` | Any call before `initialize` | "The Registry contract has not been initialized yet. Contact the operator." |
| 3 | `Unauthorized` | Caller is not the contract admin | "Only the contract admin can perform this action." |
| 4 | `NotConfigured` | A write call was made before the authorized writer contracts (Subscription, Plan) were set | "The Registry has not been configured with writer contracts yet. Contact the operator." |
| 5 | `AlreadyConfigured` | The writer contracts have already been set (they can only be set once) | "The Registry writer contracts are already configured and cannot be changed." |
| 6 | `SubscriptionAlreadyRegistered` | `register_subscription` called for an ID that already exists | "This subscription is already registered in the Registry." |
| 7 | `SubscriptionNotFound` | A Registry lookup or update referenced a subscription ID that is not registered | "Subscription not found in the Registry." |
| 8 | `InvalidStatus` | A status value outside 0–3 was supplied | "Invalid subscription status. Status must be 0 (Active), 1 (Paused), 2 (Cancelled), or 3 (Exhausted)." |
| 9 | `ChargedDecreased` | An update tried to set `total_charged` lower than its current value | "Total charged can only increase. Please report this as a bug." |
| 10 | `AlreadyFinal` | An update was attempted on a subscription whose status is already Cancelled or Exhausted | "This subscription is in a final state and cannot be updated." |
| 11 | `Overflow` | Checked arithmetic overflowed | "An arithmetic overflow occurred. Please report this as a bug." |
| 12 | `PlanAlreadyRegistered` | `register_plan` called for a plan ID that already exists | "This plan is already registered in the Registry." |

---

## Combined Quick-Lookup

Use this table to identify an error when you know only the contract name and numeric code.

| Contract | Code | Name |
|----------|-----:|------|
| Subscription | 1 | `AlreadyInitialized` |
| Subscription | 2 | `NotInitialized` |
| Subscription | 3 | `Unauthorized` |
| Subscription | 4 | `InvalidAmount` |
| Subscription | 5 | `InvalidInterval` |
| Subscription | 6 | `InvalidCap` |
| Subscription | 7 | `SameParty` |
| Subscription | 8 | `SubscriptionNotFound` |
| Subscription | 9 | `NotMerchant` |
| Subscription | 10 | `NotSubscriber` |
| Subscription | 11 | `IntervalNotElapsed` |
| Subscription | 12 | `CapExceeded` |
| Subscription | 13 | `Overflow` |
| Subscription | 14 | `SubscriptionCancelled` |
| Subscription | 15 | `SubscriptionPaused` |
| Subscription | 16 | `SubscriptionExhausted` |
| Subscription | 17 | `AlreadyPaused` |
| Subscription | 18 | `NotPaused` |
| Subscription | 19 | `TransferFailed` |
| Subscription | 20 | `ApprovalFailed` |
| Subscription | 21 | `PlanNotFound` |
| Subscription | 22 | `PlanInactive` |
| Subscription | 23 | `PlanMismatch` |
| Subscription | 24 | `AlreadyConfigured` |
| Subscription | 25 | `PlanContractNotSet` |
| Subscription | 26 | `RegistryUpdateFailed` |
| Subscription | 27 | `InvalidExpiry` |
| Subscription | 28 | `SubscriptionExpired` |
| Plan | 1 | `AlreadyInitialized` |
| Plan | 2 | `NotInitialized` |
| Plan | 3 | `Unauthorized` |
| Plan | 4 | `InvalidName` |
| Plan | 5 | `InvalidAmount` |
| Plan | 6 | `InvalidInterval` |
| Plan | 7 | `InvalidCap` |
| Plan | 8 | `PlanNotFound` |
| Plan | 9 | `NotPlanMerchant` |
| Plan | 10 | `PlanAlreadyInactive` |
| Plan | 11 | `Overflow` |
| Plan | 12 | `AlreadyConfigured` |
| Plan | 13 | `RegistryUpdateFailed` |
| Registry | 1 | `AlreadyInitialized` |
| Registry | 2 | `NotInitialized` |
| Registry | 3 | `Unauthorized` |
| Registry | 4 | `NotConfigured` |
| Registry | 5 | `AlreadyConfigured` |
| Registry | 6 | `SubscriptionAlreadyRegistered` |
| Registry | 7 | `SubscriptionNotFound` |
| Registry | 8 | `InvalidStatus` |
| Registry | 9 | `ChargedDecreased` |
| Registry | 10 | `AlreadyFinal` |
| Registry | 11 | `Overflow` |
| Registry | 12 | `PlanAlreadyRegistered` |
