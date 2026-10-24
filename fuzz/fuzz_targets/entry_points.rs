#![no_main]
//! Calls subscribe, charge, cancel, pause and resume with arbitrary
//! arguments. Errors are expected and ignored; a panic that escapes the
//! contract (other than a contract error) or a broken invariant is a crash.

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};
use subscription::{SubscriptionContract, SubscriptionContractClient};

#[derive(Debug, Arbitrary)]
enum Op {
    Subscribe { amount: i128, interval: u32, cap: i128, plan_id: u64 },
    Charge { id: u64 },
    Cancel { id: u64 },
    Pause { id: u64 },
    Resume { id: u64 },
    Advance { ledgers: u32 },
}

fuzz_target!(|ops: Vec<Op>| {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(token_admin).address();
    soroban_sdk::token::StellarAssetClient::new(&env, &token).mint(&subscriber, &1_000_000_000);

    let id = env.register(SubscriptionContract, ());
    let client = SubscriptionContractClient::new(&env, &id);
    client.initialize(&admin);

    for op in ops.into_iter().take(64) {
        match op {
            Op::Subscribe { amount, interval, cap, plan_id } => {
                let _ = client.try_subscribe(
                    &subscriber, &merchant, &token, &amount, &interval, &cap, &plan_id,
                );
            }
            Op::Charge { id } => {
                let _ = client.try_charge(&merchant, &id);
            }
            Op::Cancel { id } => {
                let _ = client.try_cancel(&subscriber, &id);
            }
            Op::Pause { id } => {
                let _ = client.try_pause(&subscriber, &id);
            }
            Op::Resume { id } => {
                let _ = client.try_resume(&subscriber, &id);
            }
            Op::Advance { ledgers } => {
                env.ledger().with_mut(|l| {
                    l.sequence_number = l.sequence_number.saturating_add(ledgers % 100_000);
                });
            }
        }
    }

    // Invariant: CAP. No subscription's remaining cap may be negative.
    for sid in 1..=8u64 {
        if let Ok(Ok(rem)) = client.try_remaining_cap(&sid) {
            assert!(rem >= 0, "remaining cap went negative");
        }
    }
});
