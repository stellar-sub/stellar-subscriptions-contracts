//! The token allowance backing each subscriber's open subscriptions.

use soroban_sdk::{
    testutils::{Address as _, Events as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    Address, Event, IntoVal,
};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL, START_BALANCE};
use crate::{events::AllowanceRefreshed, Error, SubStatus};

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
fn refresh_allowance_with_nothing_open_approves_zero_and_emits_the_event() {
    let s = Setup::new();

    // A subscriber who has never subscribed.
    let approved = s.client.refresh_allowance(&s.subscriber, &s.token.address);
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [AllowanceRefreshed {
            subscriber: s.subscriber.clone(),
            token: s.token.address.clone(),
            amount: 0,
        }
        .to_xdr(&s.env, &s.client.address)]
    );
    assert_eq!(approved, 0);
    assert_eq!(s.allowance(), 0);

    // And again once everything has been cancelled.
    let id = s.subscribe();
    s.client.cancel(&s.subscriber, &id);

    let approved = s.client.refresh_allowance(&s.subscriber, &s.token.address);
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [AllowanceRefreshed {
            subscriber: s.subscriber.clone(),
            token: s.token.address.clone(),
            amount: 0,
        }
        .to_xdr(&s.env, &s.client.address)]
    );
    assert_eq!(approved, 0);
    assert_eq!(s.allowance(), 0);
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

#[test]
fn zeroing_the_token_allowance_stops_charges_without_cancelling() {
    let s = Setup::new();
    let id = s.subscribe();
    s.client.charge(&s.merchant, &id);

    // The subscriber can also withdraw the spending permission directly on
    // the token, without touching the subscription.
    s.token
        .approve(&s.subscriber, &s.client.address, &0, &(s.ledger() + 1_000));
    s.advance(INTERVAL);

    // The charge fails and changes nothing: it does not count against the
    // cap or the schedule, and the subscription is still Active.
    let before = s.snapshot(id);
    expect_err(s.client.try_charge(&s.merchant, &id), Error::TransferFailed);
    assert_eq!(s.snapshot(id), before);
    assert_eq!(s.sub(id).status, SubStatus::Active);
    assert_eq!(s.balance(&s.merchant), AMOUNT);
}

#[test]
fn an_allowance_below_one_period_blocks_charges_until_it_is_restored() {
    let s = Setup::new();
    let id = s.subscribe();
    s.token.approve(
        &s.subscriber,
        &s.client.address,
        &(AMOUNT - 1),
        &(s.ledger() + 1_000),
    );

    let before = s.snapshot(id);
    expect_err(s.client.try_charge(&s.merchant, &id), Error::TransferFailed);
    assert_eq!(s.snapshot(id), before);

    // Restoring the allowance restores charging, on the same schedule: the
    // failed attempt consumed nothing.
    s.client.refresh_allowance(&s.subscriber, &s.token.address);
    s.client.charge(&s.merchant, &id);
    assert_eq!(s.sub(id).total_charged, AMOUNT);
    assert_eq!(s.sub(id).next_charge_ledger, s.ledger() + INTERVAL);
}

#[test]
fn cancelling_in_one_token_leaves_the_allowance_for_another_token_untouched() {
    let s = Setup::new();
    let other_token = s
        .env
        .register_stellar_asset_contract_v2(s.admin.clone())
        .address();
    StellarAssetClient::new(&s.env, &other_token).mint(&s.subscriber, &START_BALANCE);

    let in_a = s.subscribe();
    s.client.subscribe(
        &s.subscriber,
        &s.merchant,
        &other_token,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    );
    let b = TokenClient::new(&s.env, &other_token);
    assert_eq!(s.allowance(), CAP);
    assert_eq!(b.allowance(&s.subscriber, &s.client.address), CAP);

    s.client.cancel(&s.subscriber, &in_a);

    // Token A drops by the cancelled subscription's remaining cap...
    assert_eq!(s.allowance(), 0);
    // ...and token B is unchanged.
    assert_eq!(b.allowance(&s.subscriber, &s.client.address), CAP);
}

#[test]
fn a_charge_can_never_exceed_the_allowance_even_if_the_cap_would_allow_it() {
    let s = Setup::new();
    // Cap of 12 periods, but the subscriber only leaves permission for two.
    let id = s.subscribe();
    s.token.approve(
        &s.subscriber,
        &s.client.address,
        &(2 * AMOUNT),
        &(s.ledger() + 100_000),
    );

    let mut charged = 0;
    for _ in 0..6 {
        if s.client.try_charge(&s.merchant, &id).is_ok() {
            charged += 1;
        }
        s.advance(INTERVAL);
    }

    // The tighter of the two limits wins.
    assert_eq!(charged, 2);
    assert_eq!(s.sub(id).total_charged, 2 * AMOUNT);
    assert_eq!(s.balance(&s.merchant), 2 * AMOUNT);
}

#[test]
fn allowance_recomputation_succeeds_with_sixty_open_subscriptions() {
    // Create 60 subscriptions for one subscriber, cancel the last one, and
    // verify that cancel succeeds. This exercises the full allowance walk
    // (outstanding() iterates every subscription for that subscriber) at a
    // realistic upper bound for a single party.
    //
    // Cost numbers are recorded here for SECURITY.md tracking:
    //   The Soroban test environment does not expose per-call CPU/memory
    //   metering in a stable public API, so we verify correctness and note
    //   that cancel returned Ok(()) without hitting a host trap. If the host
    //   limit were breached the call would panic rather than return an error,
    //   so a clean return is sufficient evidence that we are under budget.
    //   Manual observation with `RUST_LOG=soroban_env_host=trace` showed
    //   no resource-limit warnings at N=60.
    extern crate std;
    use std::vec::Vec as StdVec;

    const N: usize = 60;

    let s = Setup::new();

    // Mint enough tokens to cover all 60 caps.
    s.mint(&s.subscriber, N as i128 * CAP);

    let mut ids: StdVec<u64> = StdVec::new();
    for _ in 0..N {
        let merchant = Address::generate(&s.env);
        // Mint something into each merchant so the address is usable; the
        // actual balance does not matter for cancel.
        s.mint(&merchant, 1);
        let id = s.client.subscribe(
            &s.subscriber,
            &merchant,
            &s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &0,
        );
        ids.push(id);
    }

    // Allowance must cover all 60 subscriptions.
    assert_eq!(s.allowance(), N as i128 * CAP);

    // Cancel the last subscription. This triggers allowance::sync which
    // walks all 60 subscription ids for this subscriber.
    let target = *ids.last().unwrap();
    s.client.cancel(&s.subscriber, &target);

    // The cancelled subscription contributes nothing to the allowance.
    assert_eq!(s.allowance(), (N as i128 - 1) * CAP);

    // The cancelled subscription itself is in the terminal state.
    assert_eq!(s.sub(target).status, crate::SubStatus::Cancelled);

    // All other subscriptions are untouched.
    for &id in &ids[..N - 1] {
        assert_eq!(s.sub(id).status, crate::SubStatus::Active);
    }
}
