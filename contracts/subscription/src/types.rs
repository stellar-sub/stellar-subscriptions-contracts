use soroban_sdk::{contracttype, Address};

/// Lifecycle state of a subscription.
///
/// The discriminants are part of the public interface: the registry and
/// off-chain indexers receive the status as a `u32`, so they must never be
/// renumbered.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum SubStatus {
    /// Chargeable once per interval, within the cap.
    Active = 0,
    /// Paused by the subscriber. No charges until resumed.
    Paused = 1,
    /// Cancelled by the subscriber. Final: no further charges, ever.
    Cancelled = 2,
    /// `total_charged` has reached `total_cap`. Final.
    Exhausted = 3,
}

impl SubStatus {
    /// Cancelled and Exhausted are terminal; nothing moves a subscription out
    /// of them.
    pub fn is_final(self) -> bool {
        matches!(self, SubStatus::Cancelled | SubStatus::Exhausted)
    }
}

/// A subscriber's bounded, revocable authorization for a merchant to pull
/// `amount_per_period` of `token` once every `interval_ledgers`, never
/// exceeding `total_cap` in aggregate.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subscription {
    pub id: u64,
    pub subscriber: Address,
    pub merchant: Address,
    pub token: Address,
    /// Fixed charge taken each interval.
    pub amount_per_period: i128,
    /// Minimum number of ledgers between two successful charges.
    pub interval_ledgers: u32,
    /// Maximum ever chargeable over the life of the subscription.
    pub total_cap: i128,
    /// Running sum of all successful charges. Never exceeds `total_cap`.
    pub total_charged: i128,
    pub start_ledger: u32,
    /// Ledger of the most recent successful charge (0 if never charged).
    pub last_charge_ledger: u32,
    /// Earliest ledger at which the next charge may succeed.
    pub next_charge_ledger: u32,
    pub status: SubStatus,
    /// Plan this subscription was created from, or 0 for an ad-hoc
    /// subscription.
    pub plan_id: u64,
}
