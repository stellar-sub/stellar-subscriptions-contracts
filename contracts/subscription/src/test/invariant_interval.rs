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
fn interval_boundary_is_exact_before_and_after_resume() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);

    assert_interval_boundary_cases(&s, id);

    s.client.pause(&s.subscriber, &id);
    s.advance(2 * INTERVAL);
    s.client.resume(&s.subscriber, &id);
    assert_interval_boundary_cases(&s, id);
}

fn assert_interval_boundary_cases(s: &Setup, id: u64) {
    let cases = [(-1_i32, false), (0, true), (1, true)];
    for (offset, succeeds) in cases {
        let next = s.sub(id).next_charge_ledger;
        let ledger = if offset < 0 {
            next - (-offset) as u32
        } else {
            next + offset as u32
        };
        s.set_ledger(ledger);

        if succeeds {
            s.client.charge(&s.merchant, &id);
        } else {
            let before = s.snapshot(id);
            expect_err(
                s.client.try_charge(&s.merchant, &id),
                Error::IntervalNotElapsed,
            );
            assert_eq!(s.snapshot(id), before);
        }
    }
}

#[test]
fn interval_of_one_allows_charges_on_consecutive_ledgers_only() {
    let s = Setup::new();
    let id = s.subscribe_with(AMOUNT, 1, 3 * AMOUNT);

    let first_charge_ledger = s.ledger();
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).next_charge_ledger, first_charge_ledger + 1);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );

    s.advance(1);
    let second_charge_ledger = s.ledger();
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).next_charge_ledger, second_charge_ledger + 1);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );
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
