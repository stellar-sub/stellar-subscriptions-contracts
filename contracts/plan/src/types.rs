use soroban_sdk::{contracttype, Address, String};

/// A merchant's reusable billing terms. Subscribers subscribe to a plan and
/// choose their own cap; `default_cap` is only a suggestion.
///
/// Field names are part of the interface: the subscription contract reads
/// plans across contracts and decodes them by name.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub id: u64,
    pub merchant: Address,
    pub name: String,
    pub token: Address,
    pub amount_per_period: i128,
    pub interval_ledgers: u32,
    /// Suggested cap for subscribers.
    pub default_cap: i128,
    /// Inactive plans accept no new subscriptions. Existing subscriptions
    /// are unaffected: they are governed by what each subscriber authorized.
    pub active: bool,
    /// Ledger the plan was created at.
    pub created_at: u32,
}
