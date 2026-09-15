//! charge() is the merchant's call; cancel / pause / resume are the
//! subscriber's. These tests make sure the two are never mixed up.

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, IntoVal,
};

use super::setup::{expect_err, Setup, INTERVAL};
use crate::{Error, SubStatus};

#[test]
fn charge_by_an_address_that_is_not_the_merchant_is_rejected() {
    let s = Setup::new();
    let id = s.subscribe();
    let attacker = Address::generate(&s.env);

    let before = s.snapshot(id);
    expect_err(s.client.try_charge(&attacker, &id), Error::NotMerchant);
    // The subscriber cannot charge their own subscription either.
    expect_err(s.client.try_charge(&s.subscriber, &id), Error::NotMerchant);
    assert_eq!(s.snapshot(id), before);
    assert_eq!(s.balance(&attacker), 0);
}

#[test]
fn cancel_by_an_address_that_is_not_the_subscriber_is_rejected() {
    let s = Setup::new();
    let id = s.subscribe();
    let attacker = Address::generate(&s.env);

    expect_err(s.client.try_cancel(&attacker, &id), Error::NotSubscriber);
    // The merchant cannot cancel on the subscriber's behalf.
    expect_err(s.client.try_cancel(&s.merchant, &id), Error::NotSubscriber);
    assert_eq!(s.sub(id).status, SubStatus::Active);

    // And the subscription is still fully usable.
    s.client.charge(&s.merchant, &id);
}

#[test]
fn pause_and_resume_by_non_subscriber_are_rejected() {
    let s = Setup::new();
    let id = s.subscribe();

    expect_err(s.client.try_pause(&s.merchant, &id), Error::NotSubscriber);
    assert_eq!(s.sub(id).status, SubStatus::Active);

    s.client.pause(&s.subscriber, &id);
    expect_err(s.client.try_resume(&s.merchant, &id), Error::NotSubscriber);
    assert_eq!(s.sub(id).status, SubStatus::Paused);
}

#[test]
fn charge_requires_the_merchants_signature_not_the_subscribers() {
    let s = Setup::new();
    let id = s.subscribe();

    // Only the subscriber has signed a `charge` invocation.
    let result = s
        .client
        .mock_auths(&[MockAuth {
            address: &s.subscriber,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "charge",
                args: (s.merchant.clone(), id).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .try_charge(&s.merchant, &id);

    assert!(result.is_err());
    assert_eq!(s.balance(&s.merchant), 0);
    assert_eq!(s.sub(id).total_charged, 0);
}

#[test]
fn cancel_requires_the_subscribers_signature_not_the_merchants() {
    let s = Setup::new();
    let id = s.subscribe();

    // Only the merchant has signed a `cancel` invocation.
    let result = s
        .client
        .mock_auths(&[MockAuth {
            address: &s.merchant,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "cancel",
                args: (s.subscriber.clone(), id).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .try_cancel(&s.subscriber, &id);

    assert!(result.is_err());
    assert_eq!(s.sub(id).status, SubStatus::Active);
}

#[test]
fn charge_is_authorized_by_the_merchant_alone() {
    let s = Setup::new();
    let id = s.subscribe();
    s.advance(INTERVAL);

    s.client.charge(&s.merchant, &id);

    let auths = s.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == s.merchant));
    assert!(auths.iter().all(|(addr, _)| *addr != s.subscriber));
}
