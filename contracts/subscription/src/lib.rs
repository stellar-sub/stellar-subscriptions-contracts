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

use soroban_sdk::{contract, contractimpl, Address, Env};
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

    /// Merchant pulls one period's charge.
    pub fn charge(env: Env, merchant: Address, subscription_id: u64) -> Result<(), Error> {
        merchant.require_auth();
        storage::read_sub(&env, subscription_id)?;
        Ok(())
    }

    /// Subscriber cancels.
    pub fn cancel(env: Env, subscriber: Address, subscription_id: u64) -> Result<(), Error> {
        subscriber.require_auth();
        storage::read_sub(&env, subscription_id)?;
        Ok(())
    }

    /// Subscriber pauses charging.
    pub fn pause(env: Env, subscriber: Address, subscription_id: u64) -> Result<(), Error> {
        subscriber.require_auth();
        storage::read_sub(&env, subscription_id)?;
        Ok(())
    }

    /// Subscriber resumes charging.
    pub fn resume(env: Env, subscriber: Address, subscription_id: u64) -> Result<(), Error> {
        subscriber.require_auth();
        storage::read_sub(&env, subscription_id)?;
        Ok(())
    }

    pub fn get_subscription(env: Env, subscription_id: u64) -> Result<Subscription, Error> {
        storage::read_sub(&env, subscription_id)
    }
}

#[cfg(test)]
mod test;
