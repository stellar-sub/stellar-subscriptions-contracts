//! The registry's stats are kept up to date incrementally. After any sequence
//! of valid registrations and updates they must equal a fold over every
//! record, recomputed from scratch.

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

use crate::{
    RegistryContract, RegistryContractClient, SubscriptionRecord, STATUS_ACTIVE,
};

#[derive(Clone, Debug)]
enum Op {
    Register,
    /// Update the subscription at `target % count` to a larger total with a
    /// status code. Refused updates (for example after a final status) are
    /// ignored, so only accepted ones reach the stats.
    Update { target: u64, delta: i128, status: u32 },
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        2 => Just(Op::Register),
        3 => (any::<u64>(), 0i128..500, 0u32..4)
            .prop_map(|(target, delta, status)| Op::Update { target, delta, status }),
    ]
}

/// Every stored record, read through the public API.
fn records(client: &RegistryContractClient) -> std::vec::Vec<SubscriptionRecord> {
    client
        .get_all_subscriptions()
        .iter()
        .map(|id| client.get_subscription_record(&id))
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    #[test]
    fn stats_always_equal_a_fold_over_the_records(ops in prop::collection::vec(op(), 1..60)) {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let subs = Address::generate(&env);
        let plans = Address::generate(&env);
        let subscriber = Address::generate(&env);
        let merchant = Address::generate(&env);
        let client = RegistryContractClient::new(&env, &env.register(RegistryContract, ()));
        client.initialize(&admin);
        client.set_writers(&admin, &subs, &plans);

        for op in ops {
            match op {
                Op::Register => {
                    let id = client.get_all_subscriptions().len() as u64 + 1;
                    client.register_subscription(&id, &subscriber, &merchant);
                }
                Op::Update { target, delta, status } => {
                    let count = client.get_all_subscriptions().len() as u64;
                    if count == 0 {
                        continue;
                    }
                    let id = target % count + 1;
                    let current = client.get_subscription_record(&id).total_charged;
                    let _ = client.try_update_subscription(&id, &(current + delta), &status);
                }
            }

            let stored = records(&client);
            let stats = client.get_stats();
            prop_assert_eq!(stats.total_subscriptions, stored.len() as u64);
            prop_assert_eq!(
                stats.active_subscriptions,
                stored.iter().filter(|r| r.status == STATUS_ACTIVE).count() as u64
            );
            prop_assert_eq!(
                stats.total_charged_volume,
                stored.iter().map(|r| r.total_charged).sum::<i128>()
            );
        }
    }
}
