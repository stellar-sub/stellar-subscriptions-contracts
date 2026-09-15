//! The token allowance backing each subscriber's open subscriptions.

use soroban_sdk::{
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    token::StellarAssetClient,
    Address, Event, IntoVal,
};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL, START_BALANCE};
use crate::{events::AllowanceRefreshed, Error};

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

#[test]
fn refresh_allowance_restores_charging_after_the_allowance_is_lost() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);

    // The allowance is gone: expired, or revoked directly on the token.
    s.token
        .approve(&s.subscriber, &s.client.address, &0, &(s.ledger() + 1_000));
    s.advance(INTERVAL);

    // Charging fails, and a failed pull changes nothing.
    let before = s.snapshot(id);
    expect_err(s.client.try_charge(&s.merchant, &id), Error::TransferFailed);
    assert_eq!(s.snapshot(id), before);

    let approved = s.client.refresh_allowance(&s.subscriber, &s.token.address);
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [AllowanceRefreshed {
            subscriber: s.subscriber.clone(),
            token: s.token.address.clone(),
            amount: CAP - AMOUNT,
        }
        .to_xdr(&s.env, &s.client.address)]
    );
    assert_eq!(approved, CAP - AMOUNT);
    assert_eq!(s.allowance(), CAP - AMOUNT);

    // Charging resumes on the normal schedule.
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).total_charged, 2 * AMOUNT);
}

#[test]
fn refresh_allowance_only_counts_open_subscriptions_in_that_token() {
    let s = Setup::new();
    let other_merchant = Address::generate(&s.env);

    let cancelled = s.subscribe();
    s.client.subscribe(
        &s.subscriber,
        &other_merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    );
    s.client.cancel(&s.subscriber, &cancelled);

    // An open subscription in a different token must not inflate this one.
    let other_token = s
        .env
        .register_stellar_asset_contract_v2(s.admin.clone())
        .address();
    StellarAssetClient::new(&s.env, &other_token).mint(&s.subscriber, &START_BALANCE);
    s.client.subscribe(
        &s.subscriber,
        &s.merchant,
        &other_token,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    );

    s.token
        .approve(&s.subscriber, &s.client.address, &0, &(s.ledger() + 1_000));
    assert_eq!(
        s.client.refresh_allowance(&s.subscriber, &s.token.address),
        CAP
    );
    assert_eq!(s.allowance(), CAP);
}

#[test]
fn refresh_allowance_requires_the_subscribers_signature() {
    let s = Setup::new();
    s.subscribe();
    s.token
        .approve(&s.subscriber, &s.client.address, &0, &(s.ledger() + 1_000));

    // The merchant cannot re-approve spending from the subscriber's account.
    let result = s
        .client
        .mock_auths(&[MockAuth {
            address: &s.merchant,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "refresh_allowance",
                args: (s.subscriber.clone(), s.token.address.clone()).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .try_refresh_allowance(&s.subscriber, &s.token.address);
    assert!(result.is_err());
    assert_eq!(s.allowance(), 0);
}
