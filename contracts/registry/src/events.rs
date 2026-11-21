//! Events emitted when the registry's index changes, so off-chain indexers
//! can follow the registry without polling its storage.
//!
//! Each event's first topic is its snake_case name, followed by the
//! subscription id. Events are published only after a write succeeds.

use soroban_sdk::{contractevent, Address};

/// A subscription was indexed for the first time. Starts Active.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionRegistered {
    #[topic]
    pub subscription_id: u64,
    #[topic]
    pub subscriber: Address,
    #[topic]
    pub merchant: Address,
}

/// A subscription's indexed total and status changed.
#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionUpdated {
    #[topic]
    pub subscription_id: u64,
    pub total_charged: i128,
    pub status: u32,
}
