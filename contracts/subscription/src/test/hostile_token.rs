//! REVOCATION against a token that misbehaves.
//!
//! The token is chosen by the subscriber and merchant, not by this contract.
//! Cancelling does follow-up work on it (lowering the allowance), so these
//! tests check that a token that panics when asked cannot stop a subscriber
//! from cancelling, and that the failure is reported instead of hidden.
//!
//! Two kinds of hostile token are tested here:
//!
//! * `HostileToken` — `approve` panics when armed; `transfer_from` can also
//!   panic when armed on the transfer side (separate arm flag).
//! * `ErrorToken` — `transfer_from` returns a contract error code when armed,
//!   instead of panicking. Both should produce `TransferFailed` and leave
//!   subscription state untouched.

use soroban_sdk::{
    contract, contracterror, contractimpl, symbol_short,
    testutils::{Address as _, Events as _, Ledger},
    Address, Env, Event,
};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL};
use crate::{
    events::Cancelled, Error, SubStatus, SubscriptionContract, SubscriptionContractClient,
};

// ---------------------------------------------------------------------------
// HostileToken
// ---------------------------------------------------------------------------

/// A token whose `approve` panics once armed (approve-arm). `transfer_from`
/// can also panic independently (transfer-arm). When neither arm is set both
/// no-ops succeed, so the test controls exactly which path breaks.
#[contract]
struct HostileToken;

#[contractimpl]
impl HostileToken {
    /// Arms the `approve` side.
    pub fn arm(env: Env) {
        env.storage().instance().set(&symbol_short!("armed"), &true);
    }

    /// Arms the `transfer_from` side independently.
    pub fn arm_transfer(env: Env) {
        env.storage()
            .instance()
            .set(&symbol_short!("tArmed"), &true);
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

    pub fn transfer_from(env: Env, _spender: Address, _from: Address, _to: Address, _amount: i128) {
        let armed = env
            .storage()
            .instance()
            .get::<_, bool>(&symbol_short!("tArmed"))
            .unwrap_or(false);
        if armed {
            panic!("hostile token refuses transfer_from");
        }
    }
}

// ---------------------------------------------------------------------------
// ErrorToken — returns a contract error instead of panicking
// ---------------------------------------------------------------------------

/// Error codes returned by `ErrorToken`.
#[contracterror]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ErrorTokenError {
    /// Returned by `transfer_from` when armed.
    InsufficientBalance = 1,
}

/// A well-behaved token that signals failures via error codes. This makes it
/// reusable: any test that wants to exercise the "token returns an error
/// rather than panicking" path can import `ErrorToken` / `ErrorTokenClient`.
#[contract]
pub struct ErrorToken;

#[contractimpl]
impl ErrorToken {
    /// Arms the token to return an error on the next `transfer_from` call.
    pub fn arm(env: Env) {
        env.storage().instance().set(&symbol_short!("armed"), &true);
    }

    pub fn approve(_env: Env, _from: Address, _spender: Address, _amount: i128, _live_until: u32) {
        // Always succeeds: approve is not under test here.
    }

    pub fn transfer_from(
        env: Env,
        _spender: Address,
        _from: Address,
        _to: Address,
        _amount: i128,
    ) -> Result<(), ErrorTokenError> {
        let armed = env
            .storage()
            .instance()
            .get::<_, bool>(&symbol_short!("armed"))
            .unwrap_or(false);
        if armed {
            return Err(ErrorTokenError::InsufficientBalance);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Shared test harness for the HostileToken world
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// HostileToken tests — approve panics
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// HostileToken tests — transfer_from panics
// ---------------------------------------------------------------------------

/// Helper: build a fresh Setup whose token is a HostileToken (arm_transfer).
fn world_with_snapshot() -> (World, u64) {
    let w = world();
    let id = w.subscribe();
    (w, id)
}

#[test]
fn a_transfer_from_panic_causes_charge_to_return_transfer_failed() {
    let (w, id) = world_with_snapshot();

    // Capture state before the (attempted) charge.
    let sub_before = w.subs.get_subscription(&id);
    w.token.arm_transfer();

    expect_err(w.subs.try_charge(&w.merchant, &id), Error::TransferFailed);

    // Subscription state must be byte-for-byte identical to before.
    assert_eq!(w.subs.get_subscription(&id), sub_before);
}

#[test]
fn a_transfer_from_panic_leaves_cap_schedule_and_status_unchanged() {
    let (w, id) = world_with_snapshot();

    let sub_before = w.subs.get_subscription(&id);
    w.token.arm_transfer();

    let _ = w.subs.try_charge(&w.merchant, &id);

    let sub_after = w.subs.get_subscription(&id);
    assert_eq!(sub_before.total_charged, sub_after.total_charged);
    assert_eq!(sub_before.total_cap, sub_after.total_cap);
    assert_eq!(sub_before.next_charge_ledger, sub_after.next_charge_ledger);
    assert_eq!(sub_before.status, sub_after.status);
}

// ---------------------------------------------------------------------------
// ErrorToken tests — transfer_from returns an error code
// ---------------------------------------------------------------------------

#[test]
fn a_transfer_from_error_causes_charge_to_return_transfer_failed() {
    let s = Setup::new();

    // Replace the real SAC token with our error-returning mock.
    let error_token = ErrorTokenClient::new(&s.env, &s.env.register(ErrorToken, ()));
    let id = s.client.subscribe(
        &s.subscriber,
        &s.merchant,
        &error_token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    );

    // Snapshot everything charge touches before arming.
    // We build the snapshot manually because Setup::snapshot uses its own
    // token address; here the token is the error mock.
    let sub_before = s.client.get_subscription(&id);

    error_token.arm();

    expect_err(s.client.try_charge(&s.merchant, &id), Error::TransferFailed);

    // State is unchanged.
    assert_eq!(s.client.get_subscription(&id), sub_before);
}

#[test]
fn a_transfer_from_error_leaves_cap_schedule_and_status_unchanged() {
    let s = Setup::new();
    let error_token = ErrorTokenClient::new(&s.env, &s.env.register(ErrorToken, ()));
    let id = s.client.subscribe(
        &s.subscriber,
        &s.merchant,
        &error_token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    );

    let sub_before = s.client.get_subscription(&id);
    error_token.arm();
    let _ = s.client.try_charge(&s.merchant, &id);
    let sub_after = s.client.get_subscription(&id);

    assert_eq!(sub_before.total_charged, sub_after.total_charged);
    assert_eq!(sub_before.total_cap, sub_after.total_cap);
    assert_eq!(sub_before.next_charge_ledger, sub_after.next_charge_ledger);
    assert_eq!(sub_before.status, sub_after.status);
}
