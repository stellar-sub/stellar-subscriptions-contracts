//! subscribe, charge, cancel, pause and resume each emit exactly one event.
//! Token events (approve, transfer) are filtered out by contract address.

use soroban_sdk::{testutils::Events as _, Event};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL};
use crate::{
    events::{Cancelled, Charged, Paused, Resumed, Subscribed},
    Error,
};

/// The last invocation emitted exactly `expected` from this contract.
fn assert_only_event<E: Event>(s: &Setup, expected: E) {
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [expected.to_xdr(&s.env, &s.client.address)]
    );
}

#[test]
fn subscribe_emits_subscribed() {
    let s = Setup::new();
    let id = s.subscribe();
    assert_only_event(
        &s,
        Subscribed {
            subscription_id: id,
            subscriber: s.subscriber.clone(),
            merchant: s.merchant.clone(),
            token: s.token.address.clone(),
            amount_per_period: AMOUNT,
            interval_ledgers: INTERVAL,
            total_cap: CAP,
            plan_id: 0,
        },
    );
}

#[test]
fn charge_emits_charged() {
    let s = Setup::new();
    let id = s.subscribe();
    let now = s.ledger();
    s.client.charge(&s.merchant, &id);
    assert_only_event(
        &s,
        Charged {
            subscription_id: id,
            merchant: s.merchant.clone(),
            amount: AMOUNT,
            total_charged: AMOUNT,
            next_charge_ledger: now + INTERVAL,
            exhausted: false,
        },
    );
}

#[test]
fn the_charge_that_reaches_the_cap_reports_exhaustion() {
    let s = Setup::new();
    let id = s.subscribe_with(AMOUNT, INTERVAL, AMOUNT);
    let now = s.ledger();
    s.client.charge(&s.merchant, &id);
    assert_only_event(
        &s,
        Charged {
            subscription_id: id,
            merchant: s.merchant.clone(),
            amount: AMOUNT,
            total_charged: AMOUNT,
            next_charge_ledger: now + INTERVAL,
            exhausted: true,
        },
    );
}

#[test]
fn cancel_emits_cancelled_with_the_unused_cap() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    s.client.cancel(&s.subscriber, &id);
    assert_only_event(
        &s,
        Cancelled {
            subscription_id: id,
            subscriber: s.subscriber.clone(),
            total_charged: AMOUNT,
            unused_cap: CAP - AMOUNT,
            allowance_updated: true,
        },
    );
}

#[test]
fn pause_emits_paused() {
    let s = Setup::new();
    let id = s.subscribe();
    let now = s.ledger();
    s.client.pause(&s.subscriber, &id);
    assert_only_event(
        &s,
        Paused {
            subscription_id: id,
            subscriber: s.subscriber.clone(),
            ledger: now,
        },
    );
}

#[test]
fn resume_emits_resumed_with_the_next_charge_ledger() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.pause(&s.subscriber, &id);
    s.advance(10);
    let now = s.ledger();
    s.client.resume(&s.subscriber, &id);
    // The first charge fell due while paused, so it is available now.
    assert_only_event(
        &s,
        Resumed {
            subscription_id: id,
            subscriber: s.subscriber.clone(),
            next_charge_ledger: now,
        },
    );
}

#[test]
fn a_rejected_charge_emits_nothing() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);
    expect_err(
        s.client.try_charge(&s.merchant, &id),
        Error::IntervalNotElapsed,
    );
    assert!(s
        .env
        .events()
        .all()
        .filter_by_contract(&s.client.address)
        .events()
        .is_empty());
}
