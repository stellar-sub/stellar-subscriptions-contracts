//! Authorization tests. Every state-changing function calls `require_auth`
//! on a specific address; these tests prove none was forgotten.
//!
//! - `charge` is the merchant's call; `cancel`, `pause`, `resume` and
//!   `refresh_allowance` are the subscriber's; admin functions are the
//!   admin's.
//! - A single test runs every state-changing function with **no** auth and
//!   asserts failure, so adding a new function without an auth check is
//!   easy to spot in review.

use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL, START_LEDGER};
use crate::{Error, SubStatus, SubscriptionContract, SubscriptionContractClient};

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

// ---- no-auth: every state-changing function must reject an unsigned call ----
//
// Each function below calls `require_auth` on a specific address. Providing
// no auth at all (mock_auths(&[])) makes every `require_auth` fail at the
// host level, so the call is rejected before any state changes. If a new
// state-changing function is added without a `require_auth`, adding it to
// this list is the one-line review catch.

#[test]
fn every_state_changing_function_rejects_a_call_with_no_signature() {
    let s = Setup::new();
    let id = s.subscribe();
    let other = Address::generate(&s.env);

    // charge — merchant.require_auth()
    assert!(s
        .client
        .mock_auths(&[])
        .try_charge(&s.merchant, &id)
        .is_err());

    // cancel — subscriber.require_auth() (via load_for_subscriber)
    assert!(s
        .client
        .mock_auths(&[])
        .try_cancel(&s.subscriber, &id)
        .is_err());

    // pause — subscriber.require_auth() (via load_for_subscriber)
    assert!(s
        .client
        .mock_auths(&[])
        .try_pause(&s.subscriber, &id)
        .is_err());

    // resume — subscriber.require_auth() (via load_for_subscriber)
    assert!(s
        .client
        .mock_auths(&[])
        .try_resume(&s.subscriber, &id)
        .is_err());

    // refresh_allowance — subscriber.require_auth()
    assert!(s
        .client
        .mock_auths(&[])
        .try_refresh_allowance(&s.subscriber, &s.token.address)
        .is_err());

    // subscribe — subscriber.require_auth()
    let new_sub = Address::generate(&s.env);
    assert!(s
        .client
        .mock_auths(&[])
        .try_subscribe(
            &new_sub,
            &s.merchant,
            &s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &0,
        )
        .is_err());

    // set_plan_contract — admin.require_auth() (via require_admin)
    assert!(s
        .client
        .mock_auths(&[])
        .try_set_plan_contract(&s.admin, &other)
        .is_err());

    // set_registry — admin.require_auth() (via require_admin)
    assert!(s
        .client
        .mock_auths(&[])
        .try_set_registry(&s.admin, &other)
        .is_err());

    // initialize — admin.require_auth() (fresh contract, not yet initialized)
    let fresh = s.env.register(SubscriptionContract, ());
    let fresh_client = SubscriptionContractClient::new(&s.env, &fresh);
    assert!(fresh_client
        .mock_auths(&[])
        .try_initialize(&s.admin)
        .is_err());
}

// ---- correct auth: each function succeeds with only the right party signing ----

#[test]
fn admin_functions_succeed_with_only_the_admins_signature() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(START_LEDGER);
    let admin = Address::generate(&env);
    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);

    let client = SubscriptionContractClient::new(&env, &env.register(SubscriptionContract, ()));

    // initialize — only the admin signs.
    client.initialize(&admin);
    let auths = env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == admin));
    assert!(auths
        .iter()
        .all(|(addr, _)| *addr != subscriber && *addr != merchant));

    // set_plan_contract (before any subscription) — only the admin signs.
    let plan = Address::generate(&env);
    client.set_plan_contract(&admin, &plan);
    let auths = env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == admin));
    assert!(auths
        .iter()
        .all(|(addr, _)| *addr != subscriber && *addr != merchant));

    // set_registry (before any subscription) — only the admin signs.
    let registry = Address::generate(&env);
    client.set_registry(&admin, &registry);
    let auths = env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == admin));
    assert!(auths
        .iter()
        .all(|(addr, _)| *addr != subscriber && *addr != merchant));
}

#[test]
fn subscribe_succeeds_with_only_the_subscribers_signature() {
    let s = Setup::new();
    s.client.subscribe(
        &s.subscriber,
        &s.merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    );

    let auths = s.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == s.subscriber));
    assert!(auths
        .iter()
        .all(|(addr, _)| *addr != s.merchant && *addr != s.admin));
}

#[test]
fn subscriber_functions_succeed_with_only_the_subscribers_signature() {
    let s = Setup::new();
    let id = s.subscribe();

    // pause — only the subscriber signs.
    s.client.pause(&s.subscriber, &id);
    let auths = s.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == s.subscriber));
    assert!(auths
        .iter()
        .all(|(addr, _)| *addr != s.merchant && *addr != s.admin));

    // resume — only the subscriber signs.
    s.client.resume(&s.subscriber, &id);
    let auths = s.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == s.subscriber));
    assert!(auths
        .iter()
        .all(|(addr, _)| *addr != s.merchant && *addr != s.admin));

    // refresh_allowance — only the subscriber signs.
    s.client.refresh_allowance(&s.subscriber, &s.token.address);
    let auths = s.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == s.subscriber));
    assert!(auths
        .iter()
        .all(|(addr, _)| *addr != s.merchant && *addr != s.admin));

    // cancel — only the subscriber signs.
    s.client.cancel(&s.subscriber, &id);
    let auths = s.env.auths();
    assert!(auths.iter().any(|(addr, _)| *addr == s.subscriber));
    assert!(auths
        .iter()
        .all(|(addr, _)| *addr != s.merchant && *addr != s.admin));
}
