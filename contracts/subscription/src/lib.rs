#![no_std]
//! # Subscription Contract
//!
//! Recurring pull payments on Soroban. A subscriber grants a bounded,
//! revocable authorization once; the merchant then pulls a fixed amount per
//! billing interval.

mod allowance;
mod errors;
mod events;
mod interfaces;
mod storage;
mod types;

pub use errors::Error;
pub use types::{SubStatus, Subscription};

use events::{Cancelled, Charged, Paused, Resumed, Subscribed};
use interfaces::{PlanClient, RegistryClient};
use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env, Vec};
use storage::DataKey;

#[contract]
pub struct SubscriptionContract;

#[contractimpl]
impl SubscriptionContract {
    /// Set the admin. Callable once.
    ///
    /// The admin can only link the plan and registry contracts, once each,
    /// before any subscription exists. It has no power to charge, cancel,
    /// pause or resume anything.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&env, &admin);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Admin links the plan contract that non-zero `plan_id`s are checked
    /// against. Once only, and only before the first subscription, so the
    /// terms a subscriber was checked against can never be swapped later.
    pub fn set_plan_contract(
        env: Env,
        admin: Address,
        plan_contract: Address,
    ) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;
        if storage::read_plan_contract(&env).is_some() || storage::has_subscriptions(&env) {
            return Err(Error::AlreadyConfigured);
        }
        storage::write_plan_contract(&env, &plan_contract);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Admin links the registry every lifecycle change is reported to. Once
    /// only, and only before the first subscription, so the registry never
    /// misses one.
    pub fn set_registry(env: Env, admin: Address, registry: Address) -> Result<(), Error> {
        Self::require_admin(&env, &admin)?;
        if storage::read_registry(&env).is_some() || storage::has_subscriptions(&env) {
            return Err(Error::AlreadyConfigured);
        }
        storage::write_registry(&env, &registry);
        storage::bump_instance(&env);
        Ok(())
    }

    pub fn get_plan_contract(env: Env) -> Option<Address> {
        storage::read_plan_contract(&env)
    }

    pub fn get_registry(env: Env) -> Option<Address> {
        storage::read_registry(&env)
    }

    /// Subscriber authorizes a subscription. This is where consent is granted
    /// and bounded: the merchant may later pull `amount_per_period` once per
    /// `interval_ledgers`, never more than `total_cap` in total.
    ///
    /// A non-zero `plan_id` must name an active plan whose merchant, token,
    /// amount and interval match exactly; 0 creates an ad-hoc subscription.
    /// The first charge is available immediately. Also approves this contract
    /// to spend the subscriber's outstanding cap in `token`.
    #[allow(clippy::too_many_arguments)]
    pub fn subscribe(
        env: Env,
        subscriber: Address,
        merchant: Address,
        token: Address,
        amount_per_period: i128,
        interval_ledgers: u32,
        total_cap: i128,
        plan_id: u64,
    ) -> Result<u64, Error> {
        storage::read_admin(&env)?;
        subscriber.require_auth();

        if subscriber == merchant {
            return Err(Error::SameParty);
        }
        if amount_per_period <= 0 {
            return Err(Error::InvalidAmount);
        }
        if interval_ledgers == 0 {
            return Err(Error::InvalidInterval);
        }
        if total_cap < amount_per_period {
            return Err(Error::InvalidCap);
        }
        if plan_id != 0 {
            Self::check_plan(
                &env,
                plan_id,
                &merchant,
                &token,
                amount_per_period,
                interval_ledgers,
            )?;
        }

        let id = storage::take_next_id(&env)?;
        let now = env.ledger().sequence();
        let sub = Subscription {
            id,
            subscriber,
            merchant,
            token,
            amount_per_period,
            interval_ledgers,
            total_cap,
            total_charged: 0,
            start_ledger: now,
            last_charge_ledger: 0,
            next_charge_ledger: now,
            status: SubStatus::Active,
            plan_id,
        };
        storage::write_sub(&env, &sub);
        storage::push_index(&env, DataKey::BySubscriber(sub.subscriber.clone()), id);
        storage::push_index(&env, DataKey::ByMerchant(sub.merchant.clone()), id);

        // After the write, so the new subscription's cap is included.
        allowance::sync(&env, &sub.subscriber, &sub.token)?;

        if let Some(registry) = storage::read_registry(&env) {
            let registered = RegistryClient::new(&env, &registry).try_register_subscription(
                &id,
                &sub.subscriber,
                &sub.merchant,
            );
            if !matches!(registered, Ok(Ok(()))) {
                return Err(Error::RegistryUpdateFailed);
            }
        }

        storage::bump_instance(&env);
        Subscribed {
            subscription_id: id,
            subscriber: sub.subscriber,
            merchant: sub.merchant,
            token: sub.token,
            amount_per_period,
            interval_ledgers,
            total_cap,
            plan_id,
        }
        .publish(&env);
        Ok(id)
    }

    /// Merchant pulls one period's charge. This is where all three
    /// invariants are enforced, in this order, before any funds move:
    ///
    /// - REVOCATION: only an `Active` subscription can be charged.
    /// - INTERVAL: the current ledger must be at or past
    ///   `next_charge_ledger`.
    /// - CAP: `total_charged + amount_per_period`, computed with
    ///   `checked_add`, must not exceed `total_cap`.
    ///
    /// On success `amount_per_period` moves subscriber -> merchant and the
    /// next charge is scheduled one full interval from now, so a late charge
    /// never unlocks a burst of catch-up charges.
    pub fn charge(env: Env, merchant: Address, subscription_id: u64) -> Result<(), Error> {
        merchant.require_auth();
        let mut sub = storage::read_sub(&env, subscription_id)?;
        if sub.merchant != merchant {
            return Err(Error::NotMerchant);
        }

        // REVOCATION.
        match sub.status {
            SubStatus::Active => {}
            SubStatus::Paused => return Err(Error::SubscriptionPaused),
            SubStatus::Cancelled => return Err(Error::SubscriptionCancelled),
            SubStatus::Exhausted => return Err(Error::SubscriptionExhausted),
        }

        // INTERVAL.
        let now = env.ledger().sequence();
        if now < sub.next_charge_ledger {
            return Err(Error::IntervalNotElapsed);
        }

        // CAP.
        let new_total = sub
            .total_charged
            .checked_add(sub.amount_per_period)
            .ok_or(Error::Overflow)?;
        if new_total > sub.total_cap {
            return Err(Error::CapExceeded);
        }
        let next_charge_ledger = now
            .checked_add(sub.interval_ledgers)
            .ok_or(Error::Overflow)?;

        sub.total_charged = new_total;
        sub.last_charge_ledger = now;
        sub.next_charge_ledger = next_charge_ledger;
        if new_total == sub.total_cap {
            sub.status = SubStatus::Exhausted;
        }
        storage::write_sub(&env, &sub);

        // Every check has passed. If the token refuses the transfer, the
        // error return rolls back the write above, so a failed pull never
        // counts against the cap or the schedule.
        let spender = env.current_contract_address();
        let transfer = TokenClient::new(&env, &sub.token).try_transfer_from(
            &spender,
            &sub.subscriber,
            &sub.merchant,
            &sub.amount_per_period,
        );
        if !matches!(transfer, Ok(Ok(()))) {
            return Err(Error::TransferFailed);
        }

        Self::report(&env, &sub)?;

        storage::bump_instance(&env);
        Charged {
            subscription_id,
            merchant: sub.merchant,
            amount: sub.amount_per_period,
            total_charged: sub.total_charged,
            next_charge_ledger: sub.next_charge_ledger,
            exhausted: sub.status == SubStatus::Exhausted,
        }
        .publish(&env);
        Ok(())
    }

    /// Subscriber cancels. Final: no further charge will ever succeed.
    ///
    /// Allowed from Active or Paused. The status change is what enforces
    /// REVOCATION — `charge` refuses anything that is not Active, and nothing
    /// moves a subscription out of Cancelled. Lowering the token allowance
    /// and updating the registry happen afterwards as clean-up, and neither
    /// is ever allowed to block the cancellation itself.
    pub fn cancel(env: Env, subscriber: Address, subscription_id: u64) -> Result<(), Error> {
        let mut sub = Self::load_for_subscriber(&env, &subscriber, subscription_id)?;
        match sub.status {
            SubStatus::Active | SubStatus::Paused => {}
            SubStatus::Cancelled => return Err(Error::SubscriptionCancelled),
            SubStatus::Exhausted => return Err(Error::SubscriptionExhausted),
        }

        sub.status = SubStatus::Cancelled;
        storage::write_sub(&env, &sub);

        // Cancellation must succeed even if the token or the registry refuses
        // the follow-up; each outcome is reported in the event instead.
        let allowance_updated = allowance::sync(&env, &sub.subscriber, &sub.token).is_ok();
        let registry_updated = Self::report(&env, &sub).is_ok();

        storage::bump_instance(&env);
        Cancelled {
            subscription_id,
            subscriber: sub.subscriber,
            total_charged: sub.total_charged,
            // Reporting only. total_charged <= total_cap always holds, and
            // saturating keeps this line from ever blocking a cancel.
            unused_cap: sub.total_cap.saturating_sub(sub.total_charged),
            allowance_updated,
            registry_updated,
        }
        .publish(&env);
        Ok(())
    }

    /// Subscriber pauses charging. Only an Active subscription can be paused.
    /// The cap, the charges so far and the token allowance are untouched.
    pub fn pause(env: Env, subscriber: Address, subscription_id: u64) -> Result<(), Error> {
        let mut sub = Self::load_for_subscriber(&env, &subscriber, subscription_id)?;
        match sub.status {
            SubStatus::Active => {}
            SubStatus::Paused => return Err(Error::AlreadyPaused),
            SubStatus::Cancelled => return Err(Error::SubscriptionCancelled),
            SubStatus::Exhausted => return Err(Error::SubscriptionExhausted),
        }

        sub.status = SubStatus::Paused;
        storage::write_sub(&env, &sub);
        Self::report(&env, &sub)?;
        storage::bump_instance(&env);
        Paused {
            subscription_id,
            subscriber,
            ledger: env.ledger().sequence(),
        }
        .publish(&env);
        Ok(())
    }

    /// Subscriber resumes a paused subscription.
    ///
    /// The schedule never moves earlier and paused periods are never billed:
    /// if the next charge was not yet due it stays where it was; if it fell
    /// due while paused, exactly one charge becomes available now.
    pub fn resume(env: Env, subscriber: Address, subscription_id: u64) -> Result<(), Error> {
        let mut sub = Self::load_for_subscriber(&env, &subscriber, subscription_id)?;
        match sub.status {
            SubStatus::Paused => {}
            SubStatus::Active => return Err(Error::NotPaused),
            SubStatus::Cancelled => return Err(Error::SubscriptionCancelled),
            SubStatus::Exhausted => return Err(Error::SubscriptionExhausted),
        }

        let now = env.ledger().sequence();
        sub.status = SubStatus::Active;
        sub.next_charge_ledger = sub.next_charge_ledger.max(now);
        storage::write_sub(&env, &sub);
        Self::report(&env, &sub)?;
        storage::bump_instance(&env);
        Resumed {
            subscription_id,
            subscriber,
            next_charge_ledger: sub.next_charge_ledger,
        }
        .publish(&env);
        Ok(())
    }

    pub fn get_subscription(env: Env, subscription_id: u64) -> Result<Subscription, Error> {
        storage::read_sub(&env, subscription_id)
    }

    /// Whether `charge` would pass every contract check right now: Active,
    /// interval elapsed, and one more period fits under the cap.
    ///
    /// The token is not consulted, so a charge can still fail with
    /// `TransferFailed` if the subscriber's balance or allowance is short.
    pub fn is_chargeable(env: Env, subscription_id: u64) -> Result<bool, Error> {
        let sub = storage::read_sub(&env, subscription_id)?;
        if sub.status != SubStatus::Active {
            return Ok(false);
        }
        let now = env.ledger().sequence();
        if now < sub.next_charge_ledger || now.checked_add(sub.interval_ledgers).is_none() {
            return Ok(false);
        }
        Ok(match sub.total_charged.checked_add(sub.amount_per_period) {
            Some(total) => total <= sub.total_cap,
            None => false,
        })
    }

    /// Cap that can still ever be charged: `total_cap - total_charged` while
    /// Active or Paused, and 0 once Cancelled or Exhausted.
    pub fn remaining_cap(env: Env, subscription_id: u64) -> Result<i128, Error> {
        let sub = storage::read_sub(&env, subscription_id)?;
        if sub.status.is_final() {
            return Ok(0);
        }
        sub.total_cap
            .checked_sub(sub.total_charged)
            .ok_or(Error::Overflow)
    }

    /// Every subscription a subscriber has created, oldest first.
    pub fn get_by_subscriber(env: Env, subscriber: Address) -> Result<Vec<Subscription>, Error> {
        Self::load_indexed(&env, &DataKey::BySubscriber(subscriber))
    }

    /// Every subscription payable to a merchant, oldest first.
    pub fn get_by_merchant(env: Env, merchant: Address) -> Result<Vec<Subscription>, Error> {
        Self::load_indexed(&env, &DataKey::ByMerchant(merchant))
    }

    // ---- internal helpers ----

    fn require_admin(env: &Env, admin: &Address) -> Result<(), Error> {
        let stored = storage::read_admin(env)?;
        admin.require_auth();
        if stored != *admin {
            return Err(Error::Unauthorized);
        }
        Ok(())
    }

    /// Require the subscriber's signature and that they own the
    /// subscription.
    fn load_for_subscriber(
        env: &Env,
        subscriber: &Address,
        subscription_id: u64,
    ) -> Result<Subscription, Error> {
        subscriber.require_auth();
        let sub = storage::read_sub(env, subscription_id)?;
        if sub.subscriber != *subscriber {
            return Err(Error::NotSubscriber);
        }
        Ok(sub)
    }

    /// A subscription to a plan must be for exactly that plan's terms, and
    /// the plan must still be accepting subscribers.
    fn check_plan(
        env: &Env,
        plan_id: u64,
        merchant: &Address,
        token: &Address,
        amount_per_period: i128,
        interval_ledgers: u32,
    ) -> Result<(), Error> {
        let plan_contract = storage::read_plan_contract(env).ok_or(Error::PlanContractNotSet)?;
        let plan = match PlanClient::new(env, &plan_contract).try_get_plan(&plan_id) {
            Ok(Ok(plan)) => plan,
            _ => return Err(Error::PlanNotFound),
        };
        if !plan.active {
            return Err(Error::PlanInactive);
        }
        if plan.merchant != *merchant
            || plan.token != *token
            || plan.amount_per_period != amount_per_period
            || plan.interval_ledgers != interval_ledgers
        {
            return Err(Error::PlanMismatch);
        }
        Ok(())
    }

    /// Mirror a subscription's total and status into the linked registry.
    /// A no-op when no registry is linked.
    fn report(env: &Env, sub: &Subscription) -> Result<(), Error> {
        let Some(registry) = storage::read_registry(env) else {
            return Ok(());
        };
        let updated = RegistryClient::new(env, &registry).try_update_subscription(
            &sub.id,
            &sub.total_charged,
            &(sub.status as u32),
        );
        if !matches!(updated, Ok(Ok(()))) {
            return Err(Error::RegistryUpdateFailed);
        }
        Ok(())
    }

    fn load_indexed(env: &Env, key: &DataKey) -> Result<Vec<Subscription>, Error> {
        let mut out = Vec::new(env);
        for id in storage::read_index(env, key).iter() {
            out.push_back(storage::read_sub(env, id)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod test;
