use soroban_sdk::contracterror;

/// Codes are append-only: never renumber or reuse one.
#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    /// Caller is not the configured admin.
    Unauthorized = 3,
    /// The writer contracts have not been set yet.
    NotConfigured = 4,
    /// The writer contracts can only be set once.
    AlreadyConfigured = 5,
    SubscriptionAlreadyRegistered = 6,
    SubscriptionNotFound = 7,
    /// Status must be 0 (Active) to 3 (Exhausted).
    InvalidStatus = 8,
    /// total_charged only ever grows.
    ChargedDecreased = 9,
    /// A Cancelled or Exhausted subscription cannot change.
    AlreadyFinal = 10,
    Overflow = 11,
    PlanAlreadyRegistered = 12,
}
