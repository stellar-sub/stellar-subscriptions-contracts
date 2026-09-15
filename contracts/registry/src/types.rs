use soroban_sdk::{contracttype, Address};

/// Status codes, matching the subscription contract's `SubStatus`
/// discriminants.
pub const STATUS_ACTIVE: u32 = 0;
pub const STATUS_PAUSED: u32 = 1;
pub const STATUS_CANCELLED: u32 = 2;
pub const STATUS_EXHAUSTED: u32 = 3;

pub fn is_final(status: u32) -> bool {
    status == STATUS_CANCELLED || status == STATUS_EXHAUSTED
}

/// Aggregate counters across every indexed plan and subscription.
#[contracttype]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegistryStats {
    pub total_plans: u64,
    pub total_subscriptions: u64,
    /// Subscriptions whose last reported status is Active.
    pub active_subscriptions: u64,
    /// Sum of every reported charge, across all tokens, in raw units.
    pub total_charged_volume: i128,
}

/// The contracts allowed to write to the registry.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Writers {
    pub subscription_contract: Address,
    pub plan_contract: Address,
}

/// What the registry knows about one subscription.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionRecord {
    pub id: u64,
    pub subscriber: Address,
    pub merchant: Address,
    pub total_charged: i128,
    pub status: u32,
}
