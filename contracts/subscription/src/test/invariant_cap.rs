//! CAP: the sum of all amounts charged to a subscription never exceeds its
//! authorized `total_cap`. A charge that would exceed the cap is rejected.

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL, START_BALANCE};
use crate::{Error, SubStatus};

#[test]
fn cap_rejects_charge_that_would_exceed_total_cap() {
    let s = Setup::new();
    // Two charges of 100 fit under 250; a third would reach 300.
    let id = s.subscribe_with(AMOUNT, INTERVAL, 250);
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);

    let before = s.snapshot(id);
    expect_err(s.client.try_charge(&s.merchant, &id), Error::CapExceeded);
    assert_eq!(s.snapshot(id), before);
    assert_eq!(s.sub(id).total_charged, 200);
}

#[test]
fn cap_charge_exactly_reaching_total_cap_sets_exhausted() {
    let s = Setup::new();
    let id = s.subscribe_with(AMOUNT, INTERVAL, 3 * AMOUNT);
    for _ in 0..3 {
        s.client.charge(&s.merchant, &id);
        s.advance(INTERVAL);
    }

    let sub = s.sub(id);
    assert_eq!(sub.total_charged, sub.total_cap);
    assert_eq!(sub.status, SubStatus::Exhausted);

    let before = s.snapshot(id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::SubscriptionExhausted,
    );
    assert_eq!(s.snapshot(id), before);
}

#[test]
fn cap_bounds_total_charged_over_the_whole_lifecycle() {
    let s = Setup::new();
    let id = s.subscribe();

    let mut successes: i128 = 0;
    for _ in 0..30 {
        if s.client.try_charge(&s.merchant, &id).is_ok() {
            successes += 1;
        }
        assert!(s.sub(id).total_charged <= CAP);
        s.advance(INTERVAL);
    }

    assert_eq!(successes, CAP / AMOUNT);
    assert_eq!(s.balance(&s.merchant), CAP);
    assert_eq!(s.balance(&s.subscriber), START_BALANCE - CAP);
}

#[test]
fn cap_uses_checked_arithmetic_on_total_charged() {
    let s = Setup::new();
    // Two of these sum past i128::MAX, so the second charge must be rejected
    // by checked_add rather than wrapping to a negative total.
    let amount = i128::MAX / 2 + 1;
    s.mint(&s.subscriber, amount);
    let id = s.subscribe_with(amount, INTERVAL, i128::MAX);
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);

    let before = s.snapshot(id);
    expect_err(s.client.try_charge(&s.merchant, &id), Error::Overflow);
    assert_eq!(s.snapshot(id), before);
}
