//! subscribe_until: an end ledger after which a subscription can never be
//! charged. It only narrows what the merchant may do.

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL, START_LEDGER};
use crate::Error;

fn subscribe_until(s: &Setup, expires_at: u32) -> u64 {
    s.client.subscribe_until(
        &s.subscriber,
        &s.merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
        &expires_at,
    )
}

fn subscribe_until_with_trial(s: &Setup, trial_ledgers: u32, expires_at: u32) -> u64 {
    s.client.subscribe_until(
        &s.subscriber,
        &s.merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &trial_ledgers,
        &expires_at,
    )
}

#[test]
fn charges_work_up_to_and_including_the_end_ledger() {
    let s = Setup::new();
    let end = START_LEDGER + 2 * INTERVAL;
    let id = subscribe_until(&s, end);
    assert_eq!(s.sub(id).expires_at, end);

    s.client.charge(&s.merchant, &id);
    s.advance(INTERVAL);
    s.client.charge(&s.merchant, &id);
    // Exactly at the end ledger a charge is still allowed.
    s.advance(INTERVAL);
    assert_eq!(s.ledger(), end);
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).total_charged, 3 * AMOUNT);
}

#[test]
fn no_charge_is_allowed_after_the_end_ledger() {
    let s = Setup::new();
    let end = START_LEDGER + INTERVAL;
    let id = subscribe_until(&s, end);

    s.set_ledger(end + 1);
    assert!(!s.client.is_chargeable(&id));
    let before = s.snapshot(id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::SubscriptionExpired,
    );
    assert_eq!(s.snapshot(id), before);
}

#[test]
fn an_end_ledger_that_is_not_in_the_future_is_refused() {
    let s = Setup::new();
    for expires_at in [0, START_LEDGER - 1, START_LEDGER] {
        expect_err(
            s.client.try_subscribe_until(
                &s.subscriber,
                &s.merchant,
                &s.token.address,
                &AMOUNT,
                &INTERVAL,
                &CAP,
                &0,
                &expires_at,
            ),
            Error::InvalidExpiry,
        );
    }
}

#[test]
fn a_plain_subscription_has_no_end() {
    let s = Setup::new();
    let id = s.subscribe();
    assert_eq!(s.sub(id).expires_at, 0);

    s.set_ledger(START_LEDGER + 50_000);
    assert!(s.client.is_chargeable(&id));
    s.client.charge(&s.merchant, &id);
}

#[test]
fn a_trial_that_ends_after_expiry_makes_the_first_charge_impossible() {
    let s = Setup::new();
    let end = START_LEDGER + INTERVAL;
    // The trial runs well past the subscription's expiry.
    let id = subscribe_until_with_trial(&s, 10 * INTERVAL, end);
    assert_eq!(s.sub(id).expires_at, end);

    // While the trial is still running the subscription is not chargeable.
    s.advance(INTERVAL);
    assert_eq!(s.ledger(), end);
    assert!(!s.client.is_chargeable(&id));

    // Once the trial would end, the subscription has already expired.
    s.set_ledger(end + 1);
    assert!(!s.client.is_chargeable(&id));
    let before = s.snapshot(id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::SubscriptionExpired,
    );
    assert_eq!(s.snapshot(id), before);
}

#[test]
fn a_trial_that_ends_before_expiry_allows_a_first_charge() {
    let s = Setup::new();
    let end = START_LEDGER + 10 * INTERVAL;
    let trial = 2 * INTERVAL;
    let id = subscribe_until_with_trial(&s, trial, end);
    assert_eq!(s.sub(id).expires_at, end);

    // During the trial nothing can be charged.
    s.advance(INTERVAL);
    assert!(!s.client.is_chargeable(&id));

    // After the trial ends the first charge succeeds.
    s.advance(INTERVAL);
    assert_eq!(s.ledger(), START_LEDGER + trial);
    assert!(s.client.is_chargeable(&id));
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).total_charged, AMOUNT);

    // A charge after expiry fails.
    s.set_ledger(end + 1);
    assert!(!s.client.is_chargeable(&id));
    let before = s.snapshot(id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::SubscriptionExpired,
    );
    assert_eq!(s.snapshot(id), before);
}

#[test]
fn is_chargeable_is_false_past_expiry_even_when_active_and_elapsed() {
    let s = Setup::new();
    let end = START_LEDGER + INTERVAL;
    let id = subscribe_until(&s, end);

    // Status is Active and the interval has elapsed, but we are past expiry.
    s.set_ledger(end + 1);
    assert!(!s.client.is_chargeable(&id));
}

#[test]
fn is_chargeable_is_true_at_exactly_expiry() {
    let s = Setup::new();
    let end = START_LEDGER + INTERVAL;
    let id = subscribe_until(&s, end);

    s.set_ledger(end);
    assert!(s.client.is_chargeable(&id));
}
