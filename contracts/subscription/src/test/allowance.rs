//! The token allowance backing each subscriber's open subscriptions.

use soroban_sdk::{testutils::Address as _, Address};

use super::setup::{Setup, AMOUNT, CAP, INTERVAL};

#[test]
fn subscribe_approves_exactly_the_authorized_cap() {
    let s = Setup::new();
    assert_eq!(s.allowance(), 0);
    s.subscribe();
    assert_eq!(s.allowance(), CAP);
}

#[test]
fn allowance_covers_every_open_subscription_in_the_same_token() {
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
    // A second subscription adds to the allowance rather than replacing it,
    // so it cannot starve the first.
    assert_eq!(s.allowance(), 2 * CAP);

    s.client.charge(&s.merchant, &a);
    s.client.charge(&other_merchant, &b);
    assert_eq!(s.allowance(), 2 * (CAP - AMOUNT));

    s.client.cancel(&s.subscriber, &a);
    assert_eq!(s.allowance(), CAP - AMOUNT);

    s.advance(INTERVAL);
    s.client.charge(&other_merchant, &b);
    assert_eq!(s.balance(&s.merchant), AMOUNT);
    assert_eq!(s.balance(&other_merchant), 2 * AMOUNT);
}

#[test]
fn allowance_is_consumed_exactly_by_charges_until_exhausted() {
    let s = Setup::new();
    let id = s.subscribe_with(AMOUNT, INTERVAL, 2 * AMOUNT);
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.allowance(), 0);
}

#[test]
fn pausing_keeps_the_allowance_for_when_the_subscription_resumes() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.pause(&s.subscriber, &id);
    assert_eq!(s.allowance(), CAP);
    s.client.resume(&s.subscriber, &id);
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.allowance(), CAP - AMOUNT);
}
