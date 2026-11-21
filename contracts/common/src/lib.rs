#![no_std]
#![warn(missing_docs)]
//! Constants shared by the subscription, plan and registry contracts.
//!
//! Keeping these in one place means a change to the ledger-time assumption
//! is made once, not in each contract's `storage.rs`.

/// Ledgers in one day, assuming a ledger closes about every five seconds
/// (86 400 s / 5 s = 17 280).
pub const DAY_IN_LEDGERS: u32 = 17_280;

#[cfg(test)]
mod tests {
    use super::DAY_IN_LEDGERS;

    #[test]
    fn day_is_17280_ledgers() {
        assert_eq!(DAY_IN_LEDGERS, 86_400 / 5);
    }
}
