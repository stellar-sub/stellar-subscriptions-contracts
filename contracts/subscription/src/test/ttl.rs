//! Time-to-live (TTL) behaviour of stored entries.
//!
//! Persistent subscription records expire unless their TTL is extended. These
//! tests check that reading a record extends it, so an active subscription
//! cannot be archived by accident.

use super::setup::Setup;
use crate::storage::DataKey;

/// Ledgers in one day. Matches the constant in `storage.rs`.
const DAY: u32 = 17_280;

/// The persistent TTL of a subscription record, read from the contract.
fn sub_ttl(s: &Setup, id: u64) -> u32 {
    s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&DataKey::Sub(id))
    })
}

#[test]
fn reading_a_subscription_extends_its_persistent_ttl() {
    let s = Setup::new();
    let id = s.subscribe();

    // Two days on, the record is inside the threshold window and has aged.
    s.advance(2 * DAY);
    let before = sub_ttl(&s, id);

    s.sub(id);
    let after = sub_ttl(&s, id);

    assert!(
        after > before,
        "reading the subscription did not extend its TTL ({before} -> {after})"
    );
}

#[test]
fn reading_a_subscription_does_not_shorten_its_persistent_ttl() {
    let s = Setup::new();
    let id = s.subscribe();

    s.advance(DAY);
    let before = sub_ttl(&s, id);
    s.sub(id);
    let after = sub_ttl(&s, id);

    assert!(after >= before, "TTL shrank on read ({before} -> {after})");
}
