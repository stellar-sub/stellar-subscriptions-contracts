//! A drop-in registry that works normally until it is armed to fail.
//!
//! Used to test that `cancel` (and other lifecycle calls) handle a registry
//! that refuses an update by reporting `registry_updated: false` in the event
//! rather than blocking the state change.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

/// A registry mock whose `register_subscription` and `update_subscription`
/// succeed until `arm()` is called, after which both panic.
///
/// It also stores the last reported record per subscription (status and
/// total charged) and tracks an `active_subscriptions` counter so tests can
/// assert on the registry state after lifecycle calls like `pause`/`resume`.
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

    fn status_key(subscription_id: u64) -> (soroban_sdk::Symbol, u64) {
        (symbol_short!("status"), subscription_id)
    }

    fn charged_key(subscription_id: u64) -> (soroban_sdk::Symbol, u64) {
        (symbol_short!("charged"), subscription_id)
    }

    /// Returns the last reported status for a subscription, or `None`.
    pub fn get_status(env: Env, subscription_id: u64) -> Option<u32> {
        env.storage()
            .instance()
            .get::<_, u32>(&Self::status_key(subscription_id))
    }

    /// Returns the last reported total charged for a subscription, or `None`.
    pub fn get_total_charged(env: Env, subscription_id: u64) -> Option<i128> {
        env.storage()
            .instance()
            .get::<_, i128>(&Self::charged_key(subscription_id))
    }

    /// Returns the current `active_subscriptions` counter.
    pub fn get_active_subscriptions(env: Env) -> u32 {
        env.storage()
            .instance()
            .get::<_, u32>(&symbol_short!("active"))
            .unwrap_or(0)
    }

    pub fn register_subscription(
        env: Env,
        subscription_id: u64,
        _subscriber: Address,
        _merchant: Address,
    ) {
        // The real registry requires the subscription contract's auth; satisfy
        // that expectation so mock_all_auths covers it.
        if Self::is_armed(&env) {
            panic!("mock registry refuses register_subscription");
        }
        // A freshly registered subscription is active.
        env.storage()
            .instance()
            .set(&Self::status_key(subscription_id), &1u32);
        let active = Self::get_active_subscriptions(env.clone());
        env.storage()
            .instance()
            .set(&symbol_short!("active"), &(active + 1));
    }

    pub fn update_subscription(
        env: Env,
        subscription_id: u64,
        total_charged: i128,
        status: u32,
    ) {
        if Self::is_armed(&env) {
            panic!("mock registry refuses update_subscription");
        }
        let previous = Self::get_status(env.clone(), subscription_id);
        env.storage()
            .instance()
            .set(&Self::status_key(subscription_id), &status);
        env.storage()
            .instance()
            .set(&Self::charged_key(subscription_id), &total_charged);

        // Keep the active counter in sync with status transitions.
        let active = Self::get_active_subscriptions(env.clone());
        let was_active = previous == Some(1u32);
        let is_active = status == 1u32;
        if was_active && !is_active {
            env.storage()
                .instance()
                .set(&symbol_short!("active"), &active.saturating_sub(1));
        } else if !was_active && is_active {
            env.storage()
                .instance()
                .set(&symbol_short!("active"), &(active + 1));
        }
    }
}
