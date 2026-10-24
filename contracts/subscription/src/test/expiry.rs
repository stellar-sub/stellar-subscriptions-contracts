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
