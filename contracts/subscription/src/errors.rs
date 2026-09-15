use soroban_sdk::contracterror;

/// Every failure the contract can report. Entry points return these instead
/// of panicking, so callers always get a stable, machine-readable code.
///
/// Codes are append-only: never renumber or reuse one.
#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    /// Caller is not the configured admin.
    Unauthorized = 3,
    /// `amount_per_period` must be positive.
    InvalidAmount = 4,
    /// `interval_ledgers` must be positive.
    InvalidInterval = 5,
    /// `total_cap` must be at least one period's charge.
    InvalidCap = 6,
    /// Subscriber and merchant must be different addresses.
    SameParty = 7,
    SubscriptionNotFound = 8,
    /// `charge` was called by an address other than the subscription's
    /// merchant.
    NotMerchant = 9,
    /// A subscriber-only action was called by someone else.
    NotSubscriber = 10,
    /// INTERVAL: the current ledger is before `next_charge_ledger`.
    IntervalNotElapsed = 11,
    /// CAP: the charge would push `total_charged` past `total_cap`.
    CapExceeded = 12,
    /// Checked arithmetic overflowed.
    Overflow = 13,
    /// REVOCATION: the subscription has been cancelled.
    SubscriptionCancelled = 14,
    SubscriptionPaused = 15,
    /// The cap has been fully charged.
    SubscriptionExhausted = 16,
    AlreadyPaused = 17,
    NotPaused = 18,
    /// The token refused the transfer (e.g. insufficient balance or
    /// allowance). No state was changed.
    TransferFailed = 19,
}
