//! Minimal interfaces to the plan and registry contracts, declared here so
//! this crate depends on neither at build time.

use soroban_sdk::{contractclient, contracttype, Address, Env, String};

/// Mirror of the plan contract's `Plan`. Contract types are decoded by field
/// name across contracts, so these names must match that contract exactly.
#[contracttype(export = false)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub id: u64,
    pub merchant: Address,
    pub name: String,
    pub token: Address,
    pub amount_per_period: i128,
    pub interval_ledgers: u32,
    pub default_cap: i128,
    pub active: bool,
    pub created_at: u32,
}

// The traits exist only so `contractclient` can generate the clients; the
// traits themselves are never implemented or named.
#[allow(dead_code)]
#[contractclient(name = "PlanClient")]
pub trait PlanInterface {
    fn get_plan(env: Env, plan_id: u64) -> Plan;
}

#[allow(dead_code)]
#[contractclient(name = "RegistryClient")]
pub trait RegistryInterface {
    fn register_subscription(
        env: Env,
        subscription_id: u64,
        subscriber: Address,
        merchant: Address,
    );
    fn update_subscription(env: Env, subscription_id: u64, total_charged: i128, status: u32);
}
