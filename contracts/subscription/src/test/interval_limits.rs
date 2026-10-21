//! Intervals so large that a charge could never be scheduled.

use super::setup::{expect_err, Setup, AMOUNT, CAP, START_LEDGER};
use crate::Error;

#[test]
fn subscribe_rejects_an_interval_that_could_never_be_charged() {
    let s = Setup::new();
    // `now + u32::MAX` overflows at the current ledger, so the first charge
    // would always fail with Overflow. The subscription must be refused.
    expect_err(
        s.client.try_subscribe(
            &s.subscriber,
            &s.merchant,
            &s.token.address,
            &AMOUNT,
            &u32::MAX,
            &CAP,
            &0,
        ),
        Error::InvalidInterval,
    );
}

#[test]
fn an_interval_that_fits_at_the_current_ledger_can_be_charged() {
    let s = Setup::new();
    // The largest interval that still fits: now + interval == u32::MAX.
    let interval = u32::MAX - START_LEDGER;
    let id = s.subscribe_with(AMOUNT, interval, CAP);

    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).next_charge_ledger, u32::MAX);
}
