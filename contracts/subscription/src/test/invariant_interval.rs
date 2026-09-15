//! INTERVAL: a charge is valid only once per billing interval. A second
//! charge before `next_charge_ledger` is rejected, even from the correct
//! merchant.

extern crate std;

use std::vec::Vec;

use super::setup::{expect_err, Setup, AMOUNT, INTERVAL};
use crate::Error;

#[test]
fn interval_rejects_second_charge_in_the_same_ledger() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);

    let before = s.snapshot(id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );
    assert_eq!(s.snapshot(id), before);
}

#[test]
fn interval_rejects_charge_one_ledger_before_next_charge_ledger() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);

    let next = s.sub(id).next_charge_ledger;
    assert_eq!(next, s.ledger() + INTERVAL);

    s.set_ledger(next - 1);
    let before = s.snapshot(id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );
    assert_eq!(s.snapshot(id), before);

    // Exactly at next_charge_ledger the charge is valid again.
    s.set_ledger(next);
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).total_charged, 2 * AMOUNT);
}

#[test]
fn interval_binds_the_correct_merchant_too() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL / 2);

    // The merchant is the legitimate payee and is authorized; the schedule
    // still wins.
    assert_eq!(s.sub(id).merchant, s.merchant);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );
    assert_eq!(s.balance(&s.merchant), AMOUNT);
}

#[test]
fn interval_late_charge_does_not_unlock_catch_up_charges() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);

    // The merchant skips five periods, then charges once.
    s.advance(5 * INTERVAL);
    s.client.charge(&s.merchant, &id);

    // Missed periods are not billable in a burst.
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );
    assert_eq!(s.sub(id).total_charged, 2 * AMOUNT);
    assert_eq!(s.sub(id).next_charge_ledger, s.ledger() + INTERVAL);
}

#[test]
fn interval_successful_charges_are_always_at_least_one_interval_apart() {
    let s = Setup::new();
    let id = s.subscribe();

    // Poll far more often than the interval allows.
    let step = INTERVAL / 3;
    let mut charged_at: Vec<u32> = Vec::new();
    for _ in 0..40 {
        if s.client.try_charge(&s.merchant, &id).is_ok() {
            charged_at.push(s.ledger());
        }
        s.advance(step);
    }

    assert!(charged_at.len() > 1);
    for pair in charged_at.windows(2) {
        assert!(
            pair[1] - pair[0] >= INTERVAL,
            "charges at {} and {} are closer than one interval",
            pair[0],
            pair[1]
        );
    }
}
