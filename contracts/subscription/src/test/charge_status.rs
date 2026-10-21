//! charge_status names the reason a charge would be refused, and agrees with
//! is_chargeable on whether it would succeed.

use super::setup::{Setup, AMOUNT, INTERVAL};
use crate::ChargeStatus;

/// `Chargeable` exactly when `is_chargeable` is true.
fn assert_agrees(s: &Setup, id: u64, status: ChargeStatus) {
    assert_eq!(s.client.charge_status(&id), status);
    assert_eq!(
        status == ChargeStatus::Chargeable,
        s.client.is_chargeable(&id),
        "charge_status and is_chargeable disagree"
    );
}

#[test]
fn a_fresh_subscription_is_chargeable() {
    let s = Setup::new();
    let id = s.subscribe();
    assert_agrees(&s, id, ChargeStatus::Chargeable);
}

#[test]
fn too_early_is_reported_until_the_interval_elapses() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    assert_agrees(&s, id, ChargeStatus::TooEarly);

    s.advance(INTERVAL);
    assert_agrees(&s, id, ChargeStatus::Chargeable);
}

#[test]
fn paused_and_cancelled_are_reported_as_such() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.pause(&s.subscriber, &id);
    assert_agrees(&s, id, ChargeStatus::Paused);

    s.client.cancel(&s.subscriber, &id);
    assert_agrees(&s, id, ChargeStatus::Cancelled);
}

#[test]
fn exhausted_is_reported_once_the_cap_is_reached() {
    let s = Setup::new();
    let id = s.subscribe_with(AMOUNT, INTERVAL, AMOUNT);
    s.client.charge(&s.merchant, &id);
    assert_agrees(&s, id, ChargeStatus::Exhausted);
}

#[test]
fn cap_would_be_exceeded_is_reported_before_a_charge_is_tried() {
    let s = Setup::new();
    // Two charges of 100 fit under 250. A third would reach 300.
    let id = s.subscribe_with(AMOUNT, INTERVAL, 250);
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);

    assert_agrees(&s, id, ChargeStatus::CapWouldBeExceeded);
}
