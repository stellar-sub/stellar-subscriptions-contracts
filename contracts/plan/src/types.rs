use soroban_sdk::{contracttype, Address, String};

/// A merchant's reusable billing terms. Subscribers subscribe to a plan and
/// choose their own cap; `default_cap` is only a suggestion.
///
/// Field names are part of the interface: the subscription contract reads
/// plans across contracts and decodes them by name.
// `contracttype` generates an undocumentable associated function from this
// struct; its own fields are still documented below.
#[allow(missing_docs)]
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    /// Starts at 1; 0 means "no plan" to the subscription contract.
    pub id: u64,
    /// Merchant who created the plan.
    pub merchant: Address,
    /// The plan's name, 1 to 64 bytes of UTF-8.
    pub name: String,
    /// Token the plan bills in.
    pub token: Address,
    /// Amount charged per interval.
    pub amount_per_period: i128,
    /// Billing interval, in ledgers.
    pub interval_ledgers: u32,
    /// Suggested cap for subscribers.
    pub default_cap: i128,
    /// Inactive plans accept no new subscriptions. Existing subscriptions
    /// are unaffected: they are governed by what each subscriber authorized.
    pub active: bool,
    /// Ledger the plan was created at.
    pub created_at: u32,
}
