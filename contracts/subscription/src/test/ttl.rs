//! Time-to-live (TTL) behaviour of stored entries.
//!
//! Persistent subscription records expire unless their TTL is extended. These
//! tests check that reading a record extends it, so an active subscription
//! cannot be archived by accident.

use super::setup::Setup;
use crate::storage::DataKey;
use common::DAY_IN_LEDGERS as DAY;

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

/// The instance TTL of the contract itself.
fn instance_ttl(s: &Setup) -> u32 {
    s.env.as_contract(&s.client.address, || {
        s.env.storage().instance().get_ttl()
    })
}

#[test]
fn charge_cancel_pause_and_resume_extend_instance_ttl() {
    let s = Setup::new();
    let id = s.subscribe();

    // Each step moves the clock close to the instance TTL threshold, so the
    // next state change must extend it.
    s.advance(2 * DAY);
    let before = instance_ttl(&s);
    s.client.charge(&s.merchant, &id);
    assert!(instance_ttl(&s) > before, "charge did not extend instance TTL");

    s.advance(2 * DAY);
    let before = instance_ttl(&s);
    s.client.pause(&s.subscriber, &id);
    assert!(instance_ttl(&s) > before, "pause did not extend instance TTL");

    s.advance(2 * DAY);
    let before = instance_ttl(&s);
    s.client.resume(&s.subscriber, &id);
    assert!(instance_ttl(&s) > before, "resume did not extend instance TTL");

    s.advance(2 * DAY);
    let before = instance_ttl(&s);
    s.client.cancel(&s.subscriber, &id);
    assert!(instance_ttl(&s) > before, "cancel did not extend instance TTL");
}

/// The plan, registry and subscription contracts all take their day length
/// from `common::DAY_IN_LEDGERS`. This pins it to the value the TTL budget
/// assumes, so a change to the shared constant is a deliberate, visible one.
#[test]
fn day_constant_is_shared_and_matches_ttl_assumption() {
    assert_eq!(DAY, 17_280);
    assert_eq!(7 * DAY, 120_960);
}

/// A deployment with only long-interval subscriptions can go weeks without a
/// contract call. The instance must still be live when the next charge
/// arrives, so that charge succeeds.
#[test]
fn charge_succeeds_after_25_days_with_no_contract_activity() {
    let s = Setup::new();
    let id = s.subscribe();

    s.advance(25 * DAY);

    s.client.charge(&s.merchant, &id);
    assert_eq!(s.balance(&s.merchant), super::setup::AMOUNT);
}
