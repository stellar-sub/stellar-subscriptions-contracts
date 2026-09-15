//! Events emitted on every state change, so indexers can rebuild a
//! subscription's full history without reading contract storage.
//!
//! Each event's first topic is its snake_case name (`subscribed`, `charged`,
//! ...), followed by the subscription id and the party that acted.

use soroban_sdk::{contractevent, Address};

/// A subscriber granted a new authorization.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Subscribed {
    #[topic]
    pub subscription_id: u64,
    #[topic]
    pub subscriber: Address,
    #[topic]
    pub merchant: Address,
    pub token: Address,
    pub amount_per_period: i128,
    pub interval_ledgers: u32,
    pub total_cap: i128,
    pub plan_id: u64,
}

/// A merchant pulled one period's charge.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Charged {
    #[topic]
    pub subscription_id: u64,
    #[topic]
    pub merchant: Address,
    pub amount: i128,
    pub total_charged: i128,
    pub next_charge_ledger: u32,
    /// True when this charge brought `total_charged` to `total_cap`.
    pub exhausted: bool,
}

/// A subscriber cancelled. Final.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cancelled {
    #[topic]
    pub subscription_id: u64,
    #[topic]
    pub subscriber: Address,
    pub total_charged: i128,
    /// Cap that was authorized but never charged.
    pub unused_cap: i128,
    /// Whether the token allowance was lowered to match. Cancellation takes
    /// effect regardless; this only reports the allowance clean-up.
    pub allowance_updated: bool,
}

/// A subscriber paused charging.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Paused {
    #[topic]
    pub subscription_id: u64,
    #[topic]
    pub subscriber: Address,
    pub ledger: u32,
}

/// A subscriber resumed charging.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resumed {
    #[topic]
    pub subscription_id: u64,
    #[topic]
    pub subscriber: Address,
    pub next_charge_ledger: u32,
}
