#![no_std]
//! # Registry Contract
//!
//! A read-optimised index of plans and subscriptions with aggregate stats,
//! for dashboards and indexers.
//!
//! The registry is bookkeeping only: it never moves funds and has no say in
//! whether a charge is allowed. Only the configured subscription contract
//! may register or update subscriptions, and only the configured plan
//! contract may register plans, so the stats cannot be forged by third
//! parties.

mod errors;
mod storage;
mod types;

pub use errors::Error;
pub use types::{
    RegistryStats, SubscriptionRecord, Writers, STATUS_ACTIVE, STATUS_CANCELLED, STATUS_EXHAUSTED,
    STATUS_PAUSED,
};

use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
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

    /// Admin sets the contracts allowed to write. Callable once, so the
    /// admin cannot later swap in a contract that rewrites history.
    pub fn set_writers(
        env: Env,
        admin: Address,
        subscription_contract: Address,
        plan_contract: Address,
    ) -> Result<(), Error> {
        let stored = storage::read_admin(&env)?;
        admin.require_auth();
        if stored != admin {
            return Err(Error::Unauthorized);
        }
        if storage::has_writers(&env) {
            return Err(Error::AlreadyConfigured);
        }
        storage::write_writers(
            &env,
            &Writers {
                subscription_contract,
                plan_contract,
            },
        );
        storage::bump_instance(&env);
        Ok(())
    }

    pub fn get_writers(env: Env) -> Result<Writers, Error> {
        storage::read_writers(&env)
    }

    /// Count a new plan. Only the plan contract may call this.
    pub fn register_plan(env: Env, plan_id: u64) -> Result<(), Error> {
        storage::read_writers(&env)?.plan_contract.require_auth();
        if !storage::mark_plan(&env, plan_id) {
            return Err(Error::PlanAlreadyRegistered);
        }
        let mut stats = storage::read_stats(&env);
        stats.total_plans = stats.total_plans.checked_add(1).ok_or(Error::Overflow)?;
        storage::write_stats(&env, &stats);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Index a new, Active subscription. Only the subscription contract may
    /// call this.
    pub fn register_subscription(
        env: Env,
        subscription_id: u64,
        subscriber: Address,
        merchant: Address,
    ) -> Result<(), Error> {
        storage::read_writers(&env)?
            .subscription_contract
            .require_auth();
        if storage::has_record(&env, subscription_id) {
            return Err(Error::SubscriptionAlreadyRegistered);
        }

        storage::write_record(
            &env,
            &SubscriptionRecord {
                id: subscription_id,
                subscriber,
                merchant,
                total_charged: 0,
                status: STATUS_ACTIVE,
            },
        );
        storage::push_id(&env, subscription_id);

        let mut stats = storage::read_stats(&env);
        stats.total_subscriptions = stats
            .total_subscriptions
            .checked_add(1)
            .ok_or(Error::Overflow)?;
        stats.active_subscriptions = stats
            .active_subscriptions
            .checked_add(1)
            .ok_or(Error::Overflow)?;
        storage::write_stats(&env, &stats);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Record a subscription's latest total and status. Only the
    /// subscription contract may call this.
    ///
    /// Mirrors the subscription contract's own rules so the index cannot
    /// drift into an impossible state: `total_charged` never decreases and a
    /// Cancelled or Exhausted subscription never changes again.
    pub fn update_subscription(
        env: Env,
        subscription_id: u64,
        total_charged: i128,
        status: u32,
    ) -> Result<(), Error> {
        storage::read_writers(&env)?
            .subscription_contract
            .require_auth();
        if status > STATUS_EXHAUSTED {
            return Err(Error::InvalidStatus);
        }
        let mut record = storage::read_record(&env, subscription_id)?;
        if types::is_final(record.status) {
            return Err(Error::AlreadyFinal);
        }
        if total_charged < record.total_charged {
            return Err(Error::ChargedDecreased);
        }

        let mut stats = storage::read_stats(&env);
        let delta = total_charged
            .checked_sub(record.total_charged)
            .ok_or(Error::Overflow)?;
        stats.total_charged_volume = stats
            .total_charged_volume
            .checked_add(delta)
            .ok_or(Error::Overflow)?;

        let was_active = record.status == STATUS_ACTIVE;
        let is_active = status == STATUS_ACTIVE;
        if was_active && !is_active {
            stats.active_subscriptions = stats
                .active_subscriptions
                .checked_sub(1)
                .ok_or(Error::Overflow)?;
        } else if !was_active && is_active {
            stats.active_subscriptions = stats
                .active_subscriptions
                .checked_add(1)
                .ok_or(Error::Overflow)?;
        }

        record.total_charged = total_charged;
        record.status = status;
        storage::write_record(&env, &record);
        storage::write_stats(&env, &stats);
        storage::bump_instance(&env);
        Ok(())
    }

    pub fn get_stats(env: Env) -> RegistryStats {
        storage::read_stats(&env)
    }

    pub fn get_subscription_record(
        env: Env,
        subscription_id: u64,
    ) -> Result<SubscriptionRecord, Error> {
        storage::read_record(&env, subscription_id)
    }

    /// Every registered subscription id, oldest first.
    pub fn get_all_subscriptions(env: Env) -> Vec<u64> {
        storage::read_all_ids(&env)
    }

    /// Ids of subscriptions whose last reported status is Active.
    pub fn get_active_subscriptions(env: Env) -> Result<Vec<u64>, Error> {
        let mut out = Vec::new(&env);
        for id in storage::read_all_ids(&env).iter() {
            if storage::read_record(&env, id)?.status == STATUS_ACTIVE {
                out.push_back(id);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod test;
