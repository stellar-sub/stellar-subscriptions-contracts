//! Guards against the subscription contract's `SubStatus` discriminants
//! silently drifting away from the registry's `STATUS_*` constants. The
//! registry stores status as a bare `u32` (see `SubscriptionRecord`), so a
//! renumbering on either side would corrupt the registry's view of every
//! subscription without a compile error. See
//! `docs/versioning-and-upgrades.md` for why these must stay synchronized.

use crate::types::SubStatus;

#[test]
fn sub_status_matches_registry_status_constants() {
    assert_eq!(SubStatus::Active as u32, registry::STATUS_ACTIVE);
    assert_eq!(SubStatus::Paused as u32, registry::STATUS_PAUSED);
    assert_eq!(SubStatus::Cancelled as u32, registry::STATUS_CANCELLED);
    assert_eq!(SubStatus::Exhausted as u32, registry::STATUS_EXHAUSTED);
    assert_eq!(
        SubStatus::MerchantEnded as u32,
        registry::STATUS_MERCHANT_ENDED
    );
}
