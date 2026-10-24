use soroban_sdk::contracterror;

/// Codes are append-only: never renumber or reuse one.
// `contracterror` generates an undocumentable associated function from this
// enum; its own variants are still documented below.
#[allow(missing_docs)]
#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// `initialize` was called a second time.
    AlreadyInitialized = 1,
    /// A call that requires an admin ran before `initialize`.
    NotInitialized = 2,
    /// Caller is not the configured admin.
    Unauthorized = 3,
    /// The writer contracts have not been set yet.
    NotConfigured = 4,
    /// The writer contracts can only be set once.
    AlreadyConfigured = 5,
    /// That subscription id has already been registered.
    SubscriptionAlreadyRegistered = 6,
    /// No subscription is registered with that id.
    SubscriptionNotFound = 7,
    /// Status must be 0 (Active) to 3 (Exhausted).
    InvalidStatus = 8,
    /// total_charged only ever grows.
    ChargedDecreased = 9,
    /// A Cancelled or Exhausted subscription cannot change.
    AlreadyFinal = 10,
    /// Checked arithmetic overflowed.
    Overflow = 11,
    /// That plan id has already been registered.
    PlanAlreadyRegistered = 12,
}
