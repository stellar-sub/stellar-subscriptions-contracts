//! REVOCATION against a token that misbehaves.
//!
//! The token is chosen by the subscriber and merchant, not by this contract.
//! Cancelling does follow-up work on it (lowering the allowance), so these
//! tests check that a token that panics when asked cannot stop a subscriber
//! from cancelling, and that the failure is reported instead of hidden.

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Events as _, Ledger},
    Address, Env, Event,
};

use super::setup::{expect_err, AMOUNT, CAP, INTERVAL};
use crate::{
    events::Cancelled, Error, SubStatus, SubscriptionContract, SubscriptionContractClient,
};

/// A token whose `approve` panics once armed. `transfer_from` always
/// succeeds without moving anything, so charges are not what is under test.
#[contract]
struct HostileToken;

#[contractimpl]
impl HostileToken {
    pub fn arm(env: Env) {
        env.storage().instance().set(&symbol_short!("armed"), &true);
    }

    pub fn approve(env: Env, _from: Address, _spender: Address, _amount: i128, _live_until: u32) {
        let armed = env
            .storage()
            .instance()
            .get::<_, bool>(&symbol_short!("armed"))
            .unwrap_or(false);
        if armed {
            panic!("hostile token refuses to approve");
        }
    }

    pub fn transfer_from(
        _env: Env,
        _spender: Address,
        _from: Address,
        _to: Address,
        _amount: i128,
    ) {
    }
}

struct World {
    env: Env,
    subs: SubscriptionContractClient<'static>,
    token: HostileTokenClient<'static>,
    subscriber: Address,
    merchant: Address,
}

fn world() -> World {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(10_000);
    let admin = Address::generate(&env);
    let subs = SubscriptionContractClient::new(&env, &env.register(SubscriptionContract, ()));
    subs.initialize(&admin);
    let token = HostileTokenClient::new(&env, &env.register(HostileToken, ()));
    World {
        subscriber: Address::generate(&env),
        merchant: Address::generate(&env),
        env,
        subs,
        token,
    }
}

impl World {
    fn subscribe(&self) -> u64 {
        self.subs.subscribe(
            &self.subscriber,
            &self.merchant,
            &self.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &0,
        )
    }
}

#[test]
fn a_token_that_panics_cannot_stop_the_subscriber_from_cancelling() {
    let w = world();
    let id = w.subscribe();
    w.subs.charge(&w.merchant, &id);

    // The token turns hostile after the subscription exists.
    w.token.arm();

    w.subs.cancel(&w.subscriber, &id);

    // Events describe the last call only, so read them before anything else.
    let emitted = w.env.events().all().filter_by_contract(&w.subs.address);

    assert_eq!(w.subs.get_subscription(&id).status, SubStatus::Cancelled);
    // The failed follow-up is reported, not hidden.
    assert_eq!(
        emitted,
        [Cancelled {
            subscription_id: id,
            subscriber: w.subscriber.clone(),
            total_charged: AMOUNT,
            unused_cap: CAP - AMOUNT,
            allowance_updated: false,
            registry_updated: true,
        }
        .to_xdr(&w.env, &w.subs.address)]
    );
}

#[test]
fn a_cancelled_subscription_stays_unchargeable_whatever_the_token_does() {
    let w = world();
    let id = w.subscribe();
    w.token.arm();
    w.subs.cancel(&w.subscriber, &id);

    w.env.ledger().set_sequence_number(10_000 + INTERVAL);
    expect_err(
        w.subs.try_charge(&w.merchant, &id),
        Error::SubscriptionCancelled,
    );
}

#[test]
fn a_token_that_panics_makes_subscribe_fail_cleanly_and_leaves_no_record() {
    let w = world();
    w.token.arm();

    expect_err(
        w.subs.try_subscribe(
            &w.subscriber,
            &w.merchant,
            &w.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &0,
        ),
        Error::ApprovalFailed,
    );
    assert_eq!(w.subs.get_by_subscriber(&w.subscriber).len(), 0);
}
