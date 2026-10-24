//! Initialization, subscribe parameter checks and lifecycle misuse.

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL};
use crate::{Error, SubStatus, SubscriptionContract, SubscriptionContractClient};

#[test]
fn initialize_twice_is_rejected() {
    let s = Setup::new();
    expect_err(s.client.try_initialize(&s.admin), Error::AlreadyInitialized);
    let other = Address::generate(&s.env);
    expect_err(s.client.try_initialize(&other), Error::AlreadyInitialized);
}

#[test]
fn subscribe_before_initialize_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = SubscriptionContractClient::new(&env, &env.register(SubscriptionContract, ()));
    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    expect_err(
        client.try_subscribe(&subscriber, &merchant, &token, &AMOUNT, &INTERVAL, &CAP, &0, &0),
        Error::NotInitialized,
    );
}

#[test]
fn subscribe_records_the_authorization() {
    let s = Setup::new();
    let now = s.ledger();
    let first = s.subscribe();
    let second = s.subscribe();
    assert_eq!((first, second), (1, 2));

    let sub = s.sub(first);
    assert_eq!(sub.subscriber, s.subscriber);
    assert_eq!(sub.merchant, s.merchant);
    assert_eq!(sub.token, s.token.address);
    assert_eq!(sub.amount_per_period, AMOUNT);
    assert_eq!(sub.interval_ledgers, INTERVAL);
    assert_eq!(sub.total_cap, CAP);
    assert_eq!(sub.total_charged, 0);
    assert_eq!(sub.start_ledger, now);
    assert_eq!(sub.last_charge_ledger, 0);
    assert_eq!(sub.next_charge_ledger, now);
    assert_eq!(sub.status, SubStatus::Active);
    assert_eq!(sub.plan_id, 0);
}

#[test]
fn ids_are_sequential_and_unique_across_many_subscribers() {
    let s = Setup::new();
    let subscribers: std::vec::Vec<Address> =
        (0..5).map(|_| Address::generate(&s.env)).collect();
    let merchants: std::vec::Vec<Address> = (0..3).map(|_| Address::generate(&s.env)).collect();

    // Every subscription gets a distinct amount so each record is identifiable.
    let mut created = std::vec::Vec::new();
    for i in 0..25usize {
        let subscriber = &subscribers[i % subscribers.len()];
        let merchant = &merchants[i % merchants.len()];
        let amount = AMOUNT + i as i128;
        let id = s.client.subscribe(
            subscriber,
            merchant,
            &s.token.address,
            &amount,
            &INTERVAL,
            &(12 * amount),
            &0,
        );
        created.push((id, subscriber, merchant, amount));
    }

    let ids: std::vec::Vec<u64> = created.iter().map(|(id, ..)| *id).collect();
    assert_eq!(ids, (1..=25).collect::<std::vec::Vec<u64>>());

    for (id, subscriber, merchant, amount) in created {
        let sub = s.sub(id);
        assert_eq!(&sub.subscriber, subscriber);
        assert_eq!(&sub.merchant, merchant);
        assert_eq!(sub.amount_per_period, amount);
        assert_eq!(sub.total_cap, 12 * amount);
        assert_eq!(sub.plan_id, 0);
    }
}

#[test]
fn subscribe_rejects_a_non_positive_amount() {
    let s = Setup::new();
    for amount in [0, -1, i128::MIN] {
        expect_err(
            s.client.try_subscribe(
                &s.subscriber,
                &s.merchant,
                &s.token.address,
                &amount,
                &INTERVAL,
                &CAP,
                &0,
                &0,
            ),
            Error::InvalidAmount,
        );
    }
}

#[test]
fn subscribe_rejects_a_zero_interval() {
    let s = Setup::new();
    expect_err(
        s.client.try_subscribe(
            &s.subscriber,
            &s.merchant,
            &s.token.address,
            &AMOUNT,
            &0,
            &CAP,
            &0,
            &0,
        ),
        Error::InvalidInterval,
    );
}

#[test]
fn subscribe_rejects_a_cap_below_one_period() {
    let s = Setup::new();
    for cap in [AMOUNT - 1, 0, -CAP] {
        expect_err(
            s.client.try_subscribe(
                &s.subscriber,
                &s.merchant,
                &s.token.address,
                &AMOUNT,
                &INTERVAL,
                &cap,
                &0,
                &0,
            ),
            Error::InvalidCap,
        );
    }
}

#[test]
fn subscribe_rejects_subscribing_to_yourself() {
    let s = Setup::new();
    expect_err(
        s.client.try_subscribe(
            &s.subscriber,
            &s.subscriber,
            &s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &0,
            &0,
        ),
        Error::SameParty,
    );
}

#[test]
fn subscribe_requires_the_subscribers_signature() {
    let s = Setup::new();
    // The merchant cannot sign a subscription into existence for someone
    // else.
    let result = s
        .client
        .mock_auths(&[MockAuth {
            address: &s.merchant,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "subscribe",
                args: (
                    s.subscriber.clone(),
                    s.merchant.clone(),
                    s.token.address.clone(),
                    AMOUNT,
                    INTERVAL,
                    CAP,
                    0u64,
                    0u32,
                )
                    .into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .try_subscribe(
            &s.subscriber,
            &s.merchant,
            &s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &0,
            &0,
        );
    assert!(result.is_err());
    assert_eq!(s.allowance(), 0);
}

#[test]
fn every_action_on_an_unknown_subscription_is_rejected() {
    let s = Setup::new();
    expect_err(
        s.client.try_charge(&s.merchant, &42),
        Error::SubscriptionNotFound,
    );
    expect_err(
        s.client.try_cancel(&s.subscriber, &42),
        Error::SubscriptionNotFound,
    );
    expect_err(
        s.client.try_pause(&s.subscriber, &42),
        Error::SubscriptionNotFound,
    );
    expect_err(
        s.client.try_resume(&s.subscriber, &42),
        Error::SubscriptionNotFound,
    );
}

#[test]
fn pausing_twice_and_resuming_an_active_subscription_are_rejected() {
    let s = Setup::new();
    let id = s.subscribe();
    expect_err(s.client.try_resume(&s.subscriber, &id), Error::NotPaused);
    s.client.pause(&s.subscriber, &id);
    expect_err(s.client.try_pause(&s.subscriber, &id), Error::AlreadyPaused);
}

#[test]
fn an_exhausted_subscription_is_final() {
    let s = Setup::new();
    let id = s.subscribe_with(AMOUNT, INTERVAL, AMOUNT);
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).status, SubStatus::Exhausted);

    expect_err(
        s.client.try_pause(&s.subscriber, &id),
        Error::SubscriptionExhausted,
    );
    expect_err(
        s.client.try_resume(&s.subscriber, &id),
        Error::SubscriptionExhausted,
    );
    expect_err(
        s.client.try_cancel(&s.subscriber, &id),
        Error::SubscriptionExhausted,
    );
}
