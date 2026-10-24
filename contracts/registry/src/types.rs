use soroban_sdk::{contracttype, Address};

/// Matches the subscription contract's `SubStatus::Active` discriminant.
pub const STATUS_ACTIVE: u32 = 0;
/// Matches the subscription contract's `SubStatus::Paused` discriminant.
pub const STATUS_PAUSED: u32 = 1;
/// Matches the subscription contract's `SubStatus::Cancelled` discriminant.
pub const STATUS_CANCELLED: u32 = 2;
/// Matches the subscription contract's `SubStatus::Exhausted` discriminant.
pub const STATUS_EXHAUSTED: u32 = 3;

/// Whether a status code is final (`Cancelled` or `Exhausted`), meaning the
/// record it belongs to can never change again.
pub fn is_final(status: u32) -> bool {
    status == STATUS_CANCELLED || status == STATUS_EXHAUSTED
}

/// Aggregate counters across every indexed plan and subscription.
// `contracttype` generates an undocumentable associated function from each
// of this file's structs; their own fields are still documented below.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegistryStats {
    /// Every plan ever registered.
    pub total_plans: u64,
    /// Every subscription ever registered.
    pub total_subscriptions: u64,
    /// Subscriptions whose last reported status is Active.
    pub active_subscriptions: u64,
    /// Sum of every reported charge, across all tokens, in raw units.
    pub total_charged_volume: i128,
}

/// The contracts allowed to write to the registry.
// See the `#[allow(missing_docs)]` note on `RegistryStats` above.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Writers {
    /// May register and update subscriptions.
    pub subscription_contract: Address,
    /// May register plans.
    pub plan_contract: Address,
}

/// What the registry knows about one subscription.
// See the `#[allow(missing_docs)]` note on `RegistryStats` above.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubscriptionRecord {
    /// The subscription's id in the subscription contract.
    pub id: u64,
    /// Who authorized the subscription.
    pub subscriber: Address,
    /// Who may charge the subscription.
    pub merchant: Address,
    /// Total charged so far, as last reported.
    pub total_charged: i128,
    /// Last reported status (0 Active, 1 Paused, 2 Cancelled, 3 Exhausted).
    pub status: u32,
}

/// Event payload emitted when a subscription is first registered.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Registered {
    pub id: u64,
    pub total_subscriptions: u64,
    pub active_subscriptions: u64,
}

/// Event payload emitted when an existing subscription record is updated.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Updated {
    pub id: u64,
    pub total_subscriptions: u64,
    pub active_subscriptions: u64,
    pub total_charged_volume: i128,
}
