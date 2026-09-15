//! REVOCATION: after the subscriber cancels, no further charge succeeds,
//! ever. Cancellation is final and subscriber-controlled.

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL};
use crate::{Error, SubStatus};

#[test]
fn revocation_charge_after_cancel_is_rejected() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    s.client.cancel(&s.subscriber, &id);
    s.advance(INTERVAL);

    assert_eq!(s.sub(id).status, SubStatus::Cancelled);
    let before = s.snapshot(id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::SubscriptionCancelled,
    );
    assert_eq!(s.snapshot(id), before);
}

#[test]
fn revocation_cancel_blocks_a_charge_that_is_already_due() {
    let s = Setup::new();
    let id = s.subscribe();
    // The first charge is due immediately; cancel before the merchant pulls.
    assert!(s.sub(id).next_charge_ledger <= s.ledger());
    s.client.cancel(&s.subscriber, &id);

    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::SubscriptionCancelled,
    );
    assert_eq!(s.balance(&s.merchant), 0);
}

#[test]
fn revocation_is_permanent() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    s.client.cancel(&s.subscriber, &id);

    for _ in 0..20 {
        s.advance(INTERVAL);
        expect_err(
            s.client.try_charge(&s.merchant, &id),
            Error::SubscriptionCancelled,
        );
    }

    // Nothing brings a cancelled subscription back.
    expect_err(
        s.client.try_resume(&s.subscriber, &id),
        Error::SubscriptionCancelled,
    );
    expect_err(
        s.client.try_pause(&s.subscriber, &id),
        Error::SubscriptionCancelled,
    );
    expect_err(
        s.client.try_cancel(&s.subscriber, &id),
        Error::SubscriptionCancelled,
    );

    assert_eq!(s.sub(id).status, SubStatus::Cancelled);
    assert_eq!(s.balance(&s.merchant), AMOUNT);
}

#[test]
fn revocation_cancelling_a_paused_subscription_is_final() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.pause(&s.subscriber, &id);
    s.client.cancel(&s.subscriber, &id);

    expect_err(
        s.client.try_resume(&s.subscriber, &id),
        Error::SubscriptionCancelled,
    );
    s.advance(INTERVAL);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::SubscriptionCancelled,
    );
    assert_eq!(s.sub(id).status, SubStatus::Cancelled);
}

#[test]
fn revocation_releases_the_unused_token_allowance() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.allowance(), CAP - AMOUNT);

    s.client.cancel(&s.subscriber, &id);
    assert_eq!(s.allowance(), 0);
}
