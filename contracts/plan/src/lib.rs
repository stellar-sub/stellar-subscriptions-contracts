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

use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, Address, Env, String, Vec,
};
use storage::DataKey;

/// Longest accepted plan name, in bytes.
pub const MAX_NAME_LEN: u32 = 64;

/// The one registry call this contract makes.
#[contractclient(name = "RegistryClient")]
pub trait RegistryInterface {
    fn register_plan(env: Env, plan_id: u64);
}

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

/// A merchant renamed a plan without changing financial terms.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanRenamed {
    #[topic]
    pub plan_id: u64,
    #[topic]
    pub merchant: Address,
    pub name: String,
}

#[contract]
pub struct PlanContract;

#[contractimpl]
impl PlanContract {
    /// Set the admin. Callable once. The admin can only link the registry.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if storage::has_admin(&env) {
            return Err(Error::AlreadyInitialized);
        }
        admin.require_auth();
        storage::write_admin(&env, &admin);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Admin links the registry every new plan is counted in. Once only, and
    /// only before the first plan, so the registry never misses one.
    pub fn set_registry(env: Env, admin: Address, registry: Address) -> Result<(), Error> {
        let stored = storage::read_admin(&env)?;
        admin.require_auth();
        if stored != admin {
            return Err(Error::Unauthorized);
        }
        if storage::read_registry(&env).is_some() || storage::has_plans(&env) {
            return Err(Error::AlreadyConfigured);
        }
        storage::write_registry(&env, &registry);
        storage::bump_instance(&env);
        Ok(())
    }

    /// Transfer admin role to a new address (can be a contract account or multisig).
    /// Only the current admin can call this.
    pub fn transfer_admin(env: Env, admin: Address, new_admin: Address) -> Result<(), Error> {
        let stored = storage::read_admin(&env)?;
        admin.require_auth();
        if stored != admin {
            return Err(Error::Unauthorized);
        }
        // New admin must authorize the transfer (works for contract accounts and multisigs)
        new_admin.require_auth();
        storage::write_admin(&env, &new_admin);
        storage::bump_instance(&env);
        Ok(())
    }

    pub fn get_registry(env: Env) -> Option<Address> {
        storage::read_registry(&env)
    }

    /// Publish a plan. Requires the merchant's signature. The plan is active
    /// immediately. `name` must be 1 to `MAX_NAME_LEN` bytes of UTF-8. The
    /// limit counts bytes, not characters, so multi-byte text uses it faster.
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

        if let Some(registry) = storage::read_registry(&env) {
            let counted = RegistryClient::new(&env, &registry).try_register_plan(&id);
            if !matches!(counted, Ok(Ok(()))) {
                return Err(Error::RegistryUpdateFailed);
            }
        }

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

    /// Rename a plan without changing its financial terms. Requires the
    /// signature of the merchant who created it. Existing subscriptions are
    /// unaffected.
    pub fn rename_plan(env: Env, merchant: Address, plan_id: u64, name: String) -> Result<(), Error> {
        merchant.require_auth();
        let mut plan = storage::read_plan(&env, plan_id)?;
        if plan.merchant != merchant {
            return Err(Error::NotPlanMerchant);
        }

        if name.is_empty() || name.len() > MAX_NAME_LEN {
            return Err(Error::InvalidName);
        }

        plan.name = name.clone();
        storage::write_plan(&env, &plan);
        storage::bump_instance(&env);
        PlanRenamed { plan_id, merchant, name }.publish(&env);
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

    /// Every plan a merchant created, paginated. Pages are contiguous and
    /// ordered by creation time (oldest first). `page_size` must be at least
    /// 1 and at most 1000.
    ///
    /// # Pagination
    /// Request the first page with `cursor = 0`. The response includes a
    /// `next_cursor`: if it is 0, there are no more pages. Pass `next_cursor`
    /// as the `cursor` for the next page.
    pub fn get_plans_by_merchant_page(
        env: Env,
        merchant: Address,
        cursor: u32,
        page_size: u32,
    ) -> Result<(Vec<Plan>, u32), Error> {
        if page_size == 0 || page_size > 1000 {
            return Err(Error::InvalidPageSize);
        }

        let ids = storage::read_index(&env, &DataKey::ByMerchant(merchant));
        let total = ids.len();
        let mut plans = Vec::new(&env);
        let mut next_cursor = 0u32;

        if cursor < total {
            let end = (cursor + page_size).min(total);
            for i in cursor..end {
                plans.push_back(storage::read_plan(&env, ids.get(i).unwrap())?);
            }
            if end < total {
                next_cursor = end;
            }
        }

        Ok((plans, next_cursor))
    }

    /// Every plan still accepting subscriptions, paginated. Pages are
    /// contiguous and ordered by creation time (oldest first). `page_size`
    /// must be at least 1 and at most 1000.
    ///
    /// # Pagination
    /// Request the first page with `cursor = 0`. The response includes a
    /// `next_cursor`: if it is 0, there are no more pages. Pass `next_cursor`
    /// as the `cursor` for the next page.
    pub fn get_active_plans_page(
        env: Env,
        cursor: u32,
        page_size: u32,
    ) -> Result<(Vec<Plan>, u32), Error> {
        if page_size == 0 || page_size > 1000 {
            return Err(Error::InvalidPageSize);
        }

        let all_ids = storage::read_index(&env, &DataKey::AllPlans);
        let mut active_plans = Vec::new(&env);
        let mut active_ids = Vec::new(&env);

        // Collect all active plan ids
        for id in all_ids.iter() {
            let plan = storage::read_plan(&env, id)?;
            if plan.active {
                active_ids.push_back(id);
            }
        }

        let total = active_ids.len();
        let mut next_cursor = 0u32;

        if cursor < total {
            let end = (cursor + page_size).min(total);
            for i in cursor..end {
                active_plans.push_back(storage::read_plan(&env, active_ids.get(i).unwrap())?);
            }
            if end < total {
                next_cursor = end;
            }
        }

        Ok((active_plans, next_cursor))
    }
}

#[cfg(test)]
mod test;
