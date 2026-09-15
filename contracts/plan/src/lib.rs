#![no_std]
//! # Plan Contract
//!
//! Merchants publish reusable billing terms — token, amount, interval and a
//! suggested cap — that subscribers subscribe to. A plan only describes
//! terms; it never moves funds and holds no authority over subscriptions.
//! Deactivating a plan stops new subscriptions to it and leaves existing
//! ones exactly as their subscribers authorized them.

mod errors;
mod storage;
mod types;

pub use errors::Error;
pub use types::Plan;

use soroban_sdk::{contract, contractevent, contractimpl, Address, Env, String, Vec};
use storage::DataKey;

/// Longest accepted plan name, in bytes.
pub const MAX_NAME_LEN: u32 = 64;

/// A merchant published a plan.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanCreated {
    #[topic]
    pub plan_id: u64,
    #[topic]
    pub merchant: Address,
    pub name: String,
    pub token: Address,
    pub amount_per_period: i128,
    pub interval_ledgers: u32,
    pub default_cap: i128,
}

/// A merchant withdrew a plan from new subscriptions.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanDeactivated {
    #[topic]
    pub plan_id: u64,
    #[topic]
    pub merchant: Address,
}

#[contract]
pub struct PlanContract;

#[contractimpl]
impl PlanContract {
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

    /// Publish a plan. Requires the merchant's signature. The plan is active
    /// immediately.
    pub fn create_plan(
        env: Env,
        merchant: Address,
        name: String,
        token: Address,
        amount_per_period: i128,
        interval_ledgers: u32,
        default_cap: i128,
    ) -> Result<u64, Error> {
        storage::read_admin(&env)?;
        merchant.require_auth();

        if name.is_empty() || name.len() > MAX_NAME_LEN {
            return Err(Error::InvalidName);
        }
        if amount_per_period <= 0 {
            return Err(Error::InvalidAmount);
        }
        if interval_ledgers == 0 {
            return Err(Error::InvalidInterval);
        }
        if default_cap < amount_per_period {
            return Err(Error::InvalidCap);
        }

        let id = storage::take_next_id(&env)?;
        let plan = Plan {
            id,
            merchant,
            name,
            token,
            amount_per_period,
            interval_ledgers,
            default_cap,
            active: true,
            created_at: env.ledger().sequence(),
        };
        storage::write_plan(&env, &plan);
        storage::push_index(&env, DataKey::ByMerchant(plan.merchant.clone()), id);
        storage::push_index(&env, DataKey::AllPlans, id);
        storage::bump_instance(&env);

        PlanCreated {
            plan_id: id,
            merchant: plan.merchant,
            name: plan.name,
            token: plan.token,
            amount_per_period,
            interval_ledgers,
            default_cap,
        }
        .publish(&env);
        Ok(id)
    }

    /// Stop accepting new subscriptions to a plan. Requires the signature of
    /// the merchant who created it. Irreversible; publish a new plan instead.
    pub fn deactivate_plan(env: Env, merchant: Address, plan_id: u64) -> Result<(), Error> {
        merchant.require_auth();
        let mut plan = storage::read_plan(&env, plan_id)?;
        if plan.merchant != merchant {
            return Err(Error::NotPlanMerchant);
        }
        if !plan.active {
            return Err(Error::PlanAlreadyInactive);
        }

        plan.active = false;
        storage::write_plan(&env, &plan);
        storage::bump_instance(&env);
        PlanDeactivated { plan_id, merchant }.publish(&env);
        Ok(())
    }

    pub fn get_plan(env: Env, plan_id: u64) -> Result<Plan, Error> {
        storage::read_plan(&env, plan_id)
    }

    /// Every plan a merchant created, active or not, oldest first.
    pub fn get_plans_by_merchant(env: Env, merchant: Address) -> Result<Vec<Plan>, Error> {
        let mut out = Vec::new(&env);
        for id in storage::read_index(&env, &DataKey::ByMerchant(merchant)).iter() {
            out.push_back(storage::read_plan(&env, id)?);
        }
        Ok(out)
    }

    /// Every plan still accepting subscriptions, oldest first.
    pub fn get_active_plans(env: Env) -> Result<Vec<Plan>, Error> {
        let mut out = Vec::new(&env);
        for id in storage::read_index(&env, &DataKey::AllPlans).iter() {
            let plan = storage::read_plan(&env, id)?;
            if plan.active {
                out.push_back(plan);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod test;
