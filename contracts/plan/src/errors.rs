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
    /// Name must be 1 to 64 bytes.
    InvalidName = 4,
    /// `amount_per_period` must be positive.
    InvalidAmount = 5,
    /// `interval_ledgers` must be positive.
    InvalidInterval = 6,
    /// `default_cap` must be at least one period's charge.
    InvalidCap = 7,
    PlanNotFound = 8,
    /// Only the merchant who created a plan can change it.
    NotPlanMerchant = 9,
    PlanAlreadyInactive = 10,
    Overflow = 11,
}
