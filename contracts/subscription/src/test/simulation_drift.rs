//! Bug under test: a `live_until_ledger` computed from the *current* ledger
//! inside the nested token `approve` drifts between simulation and
//! submission, because those two steps run several ledgers apart. Wallets
//! sign exactly the authorization tree (including its argument values) they
//! saw during simulation; if `aligned_live_until` recomputed that value from
//! `env.ledger().sequence()` at submission time instead of snapping to a
//! fixed window, the signed and submitted `approve` arguments would differ
//! and the call would trap on-chain with an auth mismatch even though
//! simulation reported success. This module asserts that the signed
//! `live_until_ledger` stays identical across a simulated `DRIFT`-ledger gap
//! for `subscribe`, `cancel` and `refresh_allowance` — the three entry
//! points whose nested `approve` the subscriber signs. It was discovered on
//! Testnet, where `subscribe` simulated cleanly and then trapped on
//! submission; see `docs/allowance-model.md` for the windowed-expiry design
//! this test guards.

use soroban_sdk::{
    testutils::{AuthorizedFunction, MockAuth, MockAuthInvoke},
    IntoVal, Symbol, TryFromVal, Val, Vec,
};

use super::setup::{Setup, AMOUNT, CAP, INTERVAL};

/// Ledgers between simulation and the transaction being applied.
const DRIFT: u32 = 3;

/// The `live_until_ledger` of the nested `approve` the subscriber was asked
/// to sign in the last invocation.
fn signed_live_until(s: &Setup) -> u32 {
    let (_, invocation) = s
        .env
        .auths()
        .into_iter()
        .find(|(addr, _)| *addr == s.subscriber)
        .expect("the subscriber authorized the last call");
    let approve = invocation
        .sub_invocations
        .first()
        .expect("a nested approve");
    match &approve.function {
        AuthorizedFunction::Contract((_, name, args)) => {
            assert_eq!(*name, Symbol::new(&s.env, "approve"));
            u32::try_from_val(&s.env, &args.get_unchecked(3)).expect("a ledger number")
        }
        _ => panic!("expected a contract call"),
    }
}

fn approve_args(s: &Setup, amount: i128, live_until: u32) -> Vec<Val> {
    (
        s.subscriber.clone(),
        s.client.address.clone(),
        amount,
        live_until,
    )
        .into_val(&s.env)
}

#[test]
fn subscribe_signed_a_few_ledgers_before_it_lands_still_succeeds() {
    // Simulate: record what the subscriber is asked to sign.
    let probe = Setup::new();
    probe.subscribe();
    let live_until = signed_live_until(&probe);

    // Submit the same subscription DRIFT ledgers later with that signature.
    let s = Setup::new();
    s.advance(DRIFT);
    let approve = MockAuthInvoke {
        contract: &s.token.address,
        fn_name: "approve",
        args: approve_args(&s, CAP, live_until),
        sub_invokes: &[],
    };
    s.client
        .mock_auths(&[MockAuth {
            address: &s.subscriber,
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
                sub_invokes: &[approve],
            },
        }])
        .subscribe(
            &s.subscriber,
            &s.merchant,
            &s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &0,
            &0,
        );
    assert_eq!(s.allowance(), CAP);
}

#[test]
fn cancel_signed_a_few_ledgers_before_it_lands_still_releases_the_allowance() {
    let probe = Setup::new();
    let probe_id = probe.subscribe();
    probe.client.cancel(&probe.subscriber, &probe_id);
    let live_until = signed_live_until(&probe);

    let s = Setup::new();
    let id = s.subscribe();
    s.advance(DRIFT);
    let approve = MockAuthInvoke {
        contract: &s.token.address,
        fn_name: "approve",
        args: approve_args(&s, 0, live_until),
        sub_invokes: &[],
    };
    s.client
        .mock_auths(&[MockAuth {
            address: &s.subscriber,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "cancel",
                args: (s.subscriber.clone(), id).into_val(&s.env),
                sub_invokes: &[approve],
            },
        }])
        .cancel(&s.subscriber, &id);
    assert_eq!(s.allowance(), 0);
}

#[test]
fn refresh_allowance_signed_a_few_ledgers_before_it_lands_still_succeeds() {
    let probe = Setup::new();
    probe.subscribe();
    probe
        .client
        .refresh_allowance(&probe.subscriber, &probe.token.address);
    let live_until = signed_live_until(&probe);

    let s = Setup::new();
    s.subscribe();
    s.advance(DRIFT);
    let approve = MockAuthInvoke {
        contract: &s.token.address,
        fn_name: "approve",
        args: approve_args(&s, CAP, live_until),
        sub_invokes: &[],
    };
    let approved = s
        .client
        .mock_auths(&[MockAuth {
            address: &s.subscriber,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "refresh_allowance",
                args: (s.subscriber.clone(), s.token.address.clone()).into_val(&s.env),
                sub_invokes: &[approve],
            },
        }])
        .refresh_allowance(&s.subscriber, &s.token.address);
    assert_eq!(approved, CAP);
}
