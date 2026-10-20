//! A drop-in registry that works normally until it is armed to fail.
//!
//! Used to test that `cancel` (and other lifecycle calls) handle a registry
//! that refuses an update by reporting `registry_updated: false` in the event
//! rather than blocking the state change.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

/// A registry mock whose `register_subscription` and `update_subscription`
/// succeed until `arm()` is called, after which both panic.
#[contract]
pub struct MockRegistry;

#[contractimpl]
impl MockRegistry {
    /// Arms the registry to panic on the next write call.
    pub fn arm(env: Env) {
        env.storage().instance().set(&symbol_short!("armed"), &true);
    }

    fn is_armed(env: &Env) -> bool {
        env.storage()
            .instance()
            .get::<_, bool>(&symbol_short!("armed"))
            .unwrap_or(false)
    }

    pub fn register_subscription(
        env: Env,
        _subscription_id: u64,
        _subscriber: Address,
        _merchant: Address,
    ) {
        // The real registry requires the subscription contract's auth; satisfy
        // that expectation so mock_all_auths covers it.
        if Self::is_armed(&env) {
            panic!("mock registry refuses register_subscription");
        }
    }

    pub fn update_subscription(
        env: Env,
        _subscription_id: u64,
        _total_charged: i128,
        _status: u32,
    ) {
        if Self::is_armed(&env) {
            panic!("mock registry refuses update_subscription");
        }
    }
}
