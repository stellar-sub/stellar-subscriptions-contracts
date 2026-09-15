#![no_std]
//! # Subscription Contract
//!
//! Recurring pull payments on Soroban. A subscriber grants a bounded,
//! revocable authorization once; the merchant then pulls a fixed amount per
//! billing interval.

mod allowance;
mod errors;
mod storage;
mod types;

pub use errors::Error;
pub use types::{SubStatus, Subscription};

use soroban_sdk::{contract, contractimpl, token::TokenClient, Address, Env};
use storage::DataKey;

#[contract]
pub struct SubscriptionContract;

#[contractimpl]
impl SubscriptionContract {
    /// Set the admin. Callable once.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&env, &admin);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Subscriber authorizes a subscription. This is where consent is granted
    /// and bounded: the merchant may later pull `amount_per_period` once per
    /// `interval_ledgers`, never more than `total_cap` in total.
    ///
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

        storage::bump_instance(&env);
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

        storage::bump_instance(&env);
        Ok(())
    }

    /// Subscriber cancels. Final: no further charge will ever succeed.
    ///
    /// Allowed from Active or Paused. The status change is what enforces
    /// REVOCATION — `charge` refuses anything that is not Active, and nothing
    /// moves a subscription out of Cancelled. Lowering the token allowance
    /// afterwards is clean-up only, and is never allowed to block the
    /// cancellation itself.
    pub fn cancel(env: Env, subscriber: Address, subscription_id: u64) -> Result<(), Error> {
        let mut sub = Self::load_for_subscriber(&env, &subscriber, subscription_id)?;
        match sub.status {
            SubStatus::Active | SubStatus::Paused => {}
            SubStatus::Cancelled => return Err(Error::SubscriptionCancelled),
            SubStatus::Exhausted => return Err(Error::SubscriptionExhausted),
        }

        sub.status = SubStatus::Cancelled;
        storage::write_sub(&env, &sub);

        // Cancellation must succeed even if the token refuses the new
        // allowance.
        let _ = allowance::sync(&env, &sub.subscriber, &sub.token);

        storage::bump_instance(&env);
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
        storage::bump_instance(&env);
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
        storage::bump_instance(&env);
        Ok(())
    }

    pub fn get_subscription(env: Env, subscription_id: u64) -> Result<Subscription, Error> {
        storage::read_sub(&env, subscription_id)
    }
}

#[cfg(test)]
mod test;
