//! remaining_cap, is_chargeable and the per-party listings.

use soroban_sdk::{testutils::Address as _, Address};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL};
use crate::{Error, SubStatus};

#[test]
fn remaining_cap_and_is_chargeable_track_the_lifecycle() {
    let s = Setup::new();
    let id = s.subscribe();

    // Fresh: the first charge is due immediately.
    assert_eq!(s.client.remaining_cap(&id), CAP);
    assert!(s.client.is_chargeable(&id));

    s.client.charge(&s.merchant, &id);
    assert_eq!(s.client.remaining_cap(&id), CAP - AMOUNT);
    assert!(!s.client.is_chargeable(&id));

    s.advance(INTERVAL - 1);
    assert!(!s.client.is_chargeable(&id));
    s.advance(1);
    assert!(s.client.is_chargeable(&id));

    // Pausing blocks charging but does not touch the cap.
    s.client.pause(&s.subscriber, &id);
    assert!(!s.client.is_chargeable(&id));
    assert_eq!(s.client.remaining_cap(&id), CAP - AMOUNT);

    s.client.resume(&s.subscriber, &id);
    assert!(s.client.is_chargeable(&id));

    // Charge out the rest of the cap, trusting is_chargeable each period.
    while s.client.is_chargeable(&id) {
        s.client.charge(&s.merchant, &id);
        s.advance(INTERVAL);
    }
    assert_eq!(s.client.remaining_cap(&id), 0);
    assert!(!s.client.is_chargeable(&id));
    assert_eq!(s.sub(id).status, SubStatus::Exhausted);
    assert_eq!(s.balance(&s.merchant), CAP);
}

#[test]
fn views_report_nothing_left_after_cancel() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);
    assert!(s.client.is_chargeable(&id));

    s.client.cancel(&s.subscriber, &id);
    assert!(!s.client.is_chargeable(&id));
    assert_eq!(s.client.remaining_cap(&id), 0);
}

#[test]
fn is_chargeable_is_false_when_the_next_charge_would_breach_the_cap() {
    let s = Setup::new();
    let id = s.subscribe_with(AMOUNT, INTERVAL, 250);
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);

    // 50 of cap remains, but a whole period does not fit.
    assert_eq!(s.client.remaining_cap(&id), 50);
    assert!(!s.client.is_chargeable(&id));
    expect_err(s.client.try_charge(&s.merchant, &id), Error::CapExceeded);
}

#[test]
fn is_chargeable_agrees_with_charge_at_every_step() {
    let s = Setup::new();
    let id = s.subscribe();

    for step in 0..60u32 {
        if step == 10 {
            s.client.pause(&s.subscriber, &id);
        }
        if step == 14 {
            s.client.resume(&s.subscriber, &id);
        }
        let predicted = s.client.is_chargeable(&id);
        let succeeded = s.client.try_charge(&s.merchant, &id).is_ok();
        assert_eq!(predicted, succeeded, "disagreement at step {step}");
        s.advance(INTERVAL / 2);
    }
}

#[test]
fn views_on_an_unknown_subscription_return_not_found() {
    let s = Setup::new();
    expect_err(s.client.try_get_subscription(&7), Error::SubscriptionNotFound);
    expect_err(s.client.try_remaining_cap(&7), Error::SubscriptionNotFound);
    expect_err(s.client.try_is_chargeable(&7), Error::SubscriptionNotFound);
}

#[test]
fn get_by_subscriber_and_get_by_merchant_list_each_partys_subscriptions() {
    let s = Setup::new();
    let other_merchant = Address::generate(&s.env);

    let a = s.subscribe();
    let b = s.client.subscribe(
        &s.subscriber,
        &other_merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    );

    let mine = s.client.get_by_subscriber(&s.subscriber);
    assert_eq!(mine.len(), 2);
    assert_eq!(mine.get_unchecked(0).id, a);
    assert_eq!(mine.get_unchecked(1).id, b);

    let first = s.client.get_by_merchant(&s.merchant);
    assert_eq!(first.len(), 1);
    assert_eq!(first.get_unchecked(0).id, a);

    let second = s.client.get_by_merchant(&other_merchant);
    assert_eq!(second.len(), 1);
    assert_eq!(second.get_unchecked(0).id, b);

    let stranger = Address::generate(&s.env);
    assert_eq!(s.client.get_by_subscriber(&stranger).len(), 0);
    assert_eq!(s.client.get_by_merchant(&stranger).len(), 0);
}

#[test]
fn listings_reflect_current_state_not_state_at_creation() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    s.client.cancel(&s.subscriber, &id);

    let listed = s.client.get_by_merchant(&s.merchant).get_unchecked(0);
    assert_eq!(listed.status, SubStatus::Cancelled);
    assert_eq!(listed.total_charged, AMOUNT);
}
