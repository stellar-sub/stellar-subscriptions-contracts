//! Charging on the happy path, and how pause / resume move the schedule.

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL, START_BALANCE};
use crate::{Error, SubStatus};

#[test]
fn charge_succeeds_once_and_transfers_exactly_amount_per_period() {
    let s = Setup::new();
    let id = s.subscribe();
    let now = s.ledger();

    s.client.charge(&s.merchant, &id);

    assert_eq!(s.balance(&s.subscriber), START_BALANCE - AMOUNT);
    assert_eq!(s.balance(&s.merchant), AMOUNT);
    // Funds move subscriber -> merchant directly; the contract never holds
    // them.
    assert_eq!(s.balance(&s.client.address), 0);

    let sub = s.sub(id);
    assert_eq!(sub.total_charged, AMOUNT);
    assert_eq!(sub.last_charge_ledger, now);
    assert_eq!(sub.next_charge_ledger, now + INTERVAL);
    assert_eq!(sub.status, SubStatus::Active);
    assert_eq!(s.allowance(), CAP - AMOUNT);
}

#[test]
fn charge_on_paused_subscription_is_rejected() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.pause(&s.subscriber, &id);
    s.advance(INTERVAL);

    let before = s.snapshot(id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::SubscriptionPaused,
    );
    assert_eq!(s.snapshot(id), before);
}

#[test]
fn resume_after_pause_allows_charging_again_on_schedule() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    s.client.pause(&s.subscriber, &id);

    // Three periods pass while paused.
    s.advance(3 * INTERVAL);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::SubscriptionPaused,
    );

    s.client.resume(&s.subscriber, &id);
    assert_eq!(s.sub(id).status, SubStatus::Active);

    // One charge is due on resume; the paused periods are not billable.
    s.client.charge(&s.merchant, &id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );

    s.advance(INTERVAL);
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).total_charged, 3 * AMOUNT);
}

#[test]
fn resume_before_due_keeps_the_original_schedule() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    let due = s.sub(id).next_charge_ledger;

    s.advance(INTERVAL / 4);
    s.client.pause(&s.subscriber, &id);
    s.advance(INTERVAL / 4);
    s.client.resume(&s.subscriber, &id);

    // Pausing never pulls the next charge earlier.
    assert_eq!(s.sub(id).next_charge_ledger, due);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );

    s.set_ledger(due);
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).total_charged, 2 * AMOUNT);
}

#[test]
fn charge_fails_cleanly_when_the_subscriber_cannot_pay() {
    let s = Setup::new();
    let id = s.subscribe();
    s.token.burn(&s.subscriber, &START_BALANCE);

    let before = s.snapshot(id);
    expect_err(s.client.try_charge(&s.merchant, &id), Error::TransferFailed);
    // A failed transfer must not count against the cap or the schedule.
    assert_eq!(s.snapshot(id), before);
}

#[test]
fn repeated_pause_and_resume_never_pulls_next_charge_earlier() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    let original_due = s.sub(id).next_charge_ledger;

    // Resume reschedules at max(next_charge_ledger, now), so every
    // pause/resume cycle can only push the due ledger forward, never back.
    let steps = [INTERVAL / 4, INTERVAL, INTERVAL / 2, 2 * INTERVAL, 1, INTERVAL + 5];
    let mut prev = original_due;
    for (i, &step) in steps.iter().cycle().take(20).enumerate() {
        s.advance(step);
        s.client.pause(&s.subscriber, &id);
        s.client.resume(&s.subscriber, &id);
        let after = s.sub(id).next_charge_ledger;
        assert!(
            after >= prev,
            "next_charge_ledger decreased at cycle {i}: {after} < {prev}"
        );
        prev = after;
    }

    // The first charge after the cycles is allowed no earlier than the
    // original schedule: the due ledger only ever moved forward.
    let final_due = s.sub(id).next_charge_ledger;
    assert!(final_due >= original_due);

    s.set_ledger(original_due.saturating_sub(1));
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );

    s.set_ledger(final_due);
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).total_charged, 2 * AMOUNT);
}
