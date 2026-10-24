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
    /// Name must be 1 to 64 bytes.
    InvalidName = 4,
    /// `amount_per_period` must be positive.
    InvalidAmount = 5,
    /// `interval_ledgers` must be positive.
    InvalidInterval = 6,
    /// `default_cap` must be at least one period's charge.
    InvalidCap = 7,
    /// No plan exists with that id.
    PlanNotFound = 8,
    /// Only the merchant who created a plan can change it.
    NotPlanMerchant = 9,
    /// `deactivate_plan` was called on a plan that is already inactive.
    PlanAlreadyInactive = 10,
    /// Checked arithmetic overflowed.
    Overflow = 11,
    /// The registry can be linked once, before any plan exists.
    AlreadyConfigured = 12,
    /// The linked registry refused to count the plan.
    RegistryUpdateFailed = 13,
}
