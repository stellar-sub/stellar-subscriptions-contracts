extern crate std;

use core::fmt::Debug;

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    Address, Env, IntoVal,
};

use crate::{
    Error, RegistryContract, RegistryContractClient, RegistryStats, SubscriptionRecord,
    STATUS_ACTIVE, STATUS_CANCELLED, STATUS_EXHAUSTED, STATUS_PAUSED,
};

struct Setup {
    env: Env,
    client: RegistryContractClient<'static>,
    admin: Address,
    /// Stands in for the subscription contract's address.
    subs: Address,
    /// Stands in for the plan contract's address.
    plans: Address,
    subscriber: Address,
    merchant: Address,
}

impl Setup {
    fn unconfigured() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let admin = Address::generate(&env);
        let client = RegistryContractClient::new(&env, &env.register(RegistryContract, ()));
        client.initialize(&admin);
        Setup {
            subs: Address::generate(&env),
            plans: Address::generate(&env),
            subscriber: Address::generate(&env),
            merchant: Address::generate(&env),
            env,
            client,
            admin,
        }
    }

    fn new() -> Self {
        let s = Self::unconfigured();
        s.client.set_writers(&s.admin, &s.subs, &s.plans);
        s
    }

    fn register(&self, id: u64) {
        self.client
            .register_subscription(&id, &self.subscriber, &self.merchant);
    }

    fn stats(&self) -> RegistryStats {
        self.client.get_stats()
    }
}

fn expect_err<T: Debug, C: Debug, I: Debug>(
    result: Result<Result<T, C>, Result<Error, I>>,
    expected: Error,
) {
    match result {
        Err(Ok(actual)) => assert_eq!(actual, expected),
        Err(Err(host)) => panic!("expected {expected:?}, got host error {host:?}"),
        Ok(value) => panic!("expected {expected:?}, but the call succeeded with {value:?}"),
    }
}

fn ids(list: soroban_sdk::Vec<u64>) -> std::vec::Vec<u64> {
    list.iter().collect()
}

#[test]
fn initialize_twice_is_rejected() {
    let s = Setup::new();
    expect_err(s.client.try_initialize(&s.admin), Error::AlreadyInitialized);
}

#[test]
fn stats_start_at_zero() {
    let s = Setup::new();
    assert_eq!(s.stats(), RegistryStats::default());
    assert!(s.client.get_all_subscriptions().is_empty());
}

#[test]
fn only_the_admin_can_set_writers_and_only_once() {
    let s = Setup::unconfigured();
    let stranger = Address::generate(&s.env);
    expect_err(
        s.client.try_set_writers(&stranger, &s.subs, &s.plans),
        Error::Unauthorized,
    );

    s.client.set_writers(&s.admin, &s.subs, &s.plans);
    assert_eq!(s.client.get_writers().subscription_contract, s.subs);

    // The admin cannot swap in a different writer later.
    expect_err(
        s.client.try_set_writers(&s.admin, &stranger, &stranger),
        Error::AlreadyConfigured,
    );
}

#[test]
fn writes_before_configuration_are_rejected() {
    let s = Setup::unconfigured();
    expect_err(
        s.client
            .try_register_subscription(&1, &s.subscriber, &s.merchant),
        Error::NotConfigured,
    );
    expect_err(s.client.try_register_plan(&1), Error::NotConfigured);
}

#[test]
fn register_subscription_counts_it_as_active() {
    let s = Setup::new();
    s.register(1);
    s.register(2);

    let stats = s.stats();
    assert_eq!(stats.total_subscriptions, 2);
    assert_eq!(stats.active_subscriptions, 2);
    assert_eq!(
        s.client.get_subscription_record(&1),
        SubscriptionRecord {
            id: 1,
            subscriber: s.subscriber.clone(),
            merchant: s.merchant.clone(),
            total_charged: 0,
            status: STATUS_ACTIVE,
        }
    );
}

#[test]
fn registering_the_same_subscription_twice_is_rejected() {
    let s = Setup::new();
    s.register(1);
    expect_err(
        s.client
            .try_register_subscription(&1, &s.subscriber, &s.merchant),
        Error::SubscriptionAlreadyRegistered,
    );
    assert_eq!(s.stats().total_subscriptions, 1);
}

#[test]
fn only_the_subscription_contract_can_register_subscriptions() {
    let s = Setup::new();
    let forger = Address::generate(&s.env);
    let result = s
        .client
        .mock_auths(&[MockAuth {
            address: &forger,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "register_subscription",
                args: (1u64, s.subscriber.clone(), s.merchant.clone()).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .try_register_subscription(&1, &s.subscriber, &s.merchant);
    assert!(result.is_err());
    assert_eq!(s.stats().total_subscriptions, 0);
}

#[test]
fn only_the_subscription_contract_can_update_subscriptions() {
    let s = Setup::new();
    s.register(1);
    let forger = Address::generate(&s.env);
    let result = s
        .client
        .mock_auths(&[MockAuth {
            address: &forger,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "update_subscription",
                args: (1u64, 1_000_000i128, STATUS_ACTIVE).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .try_update_subscription(&1, &1_000_000, &STATUS_ACTIVE);
    assert!(result.is_err());
    assert_eq!(s.stats().total_charged_volume, 0);
}

#[test]
fn charges_accumulate_volume_by_their_increase() {
    let s = Setup::new();
    s.register(1);
    s.register(2);
    s.client.update_subscription(&1, &100, &STATUS_ACTIVE);
    s.client.update_subscription(&1, &200, &STATUS_ACTIVE);
    s.client.update_subscription(&2, &50, &STATUS_ACTIVE);

    assert_eq!(s.stats().total_charged_volume, 250);
    assert_eq!(s.client.get_subscription_record(&1).total_charged, 200);
}

#[test]
fn total_charged_can_never_decrease() {
    let s = Setup::new();
    s.register(1);
    s.client.update_subscription(&1, &200, &STATUS_ACTIVE);
    expect_err(
        s.client.try_update_subscription(&1, &100, &STATUS_ACTIVE),
        Error::ChargedDecreased,
    );
    assert_eq!(s.stats().total_charged_volume, 200);
}

#[test]
fn pause_and_resume_move_the_active_count() {
    let s = Setup::new();
    s.register(1);
    s.client.update_subscription(&1, &0, &STATUS_PAUSED);
    assert_eq!(s.stats().active_subscriptions, 0);
    s.client.update_subscription(&1, &0, &STATUS_ACTIVE);
    assert_eq!(s.stats().active_subscriptions, 1);
}

#[test]
fn cancelling_a_paused_subscription_does_not_double_count() {
    let s = Setup::new();
    s.register(1);
    s.register(2);
    s.client.update_subscription(&1, &0, &STATUS_PAUSED);
    s.client.update_subscription(&1, &0, &STATUS_CANCELLED);
    s.client.update_subscription(&2, &300, &STATUS_EXHAUSTED);

    let stats = s.stats();
    assert_eq!(stats.active_subscriptions, 0);
    assert_eq!(stats.total_subscriptions, 2);
    assert_eq!(stats.total_charged_volume, 300);
}

#[test]
fn final_subscriptions_never_change_again() {
    let s = Setup::new();
    s.register(1);
    s.register(2);
    s.client.update_subscription(&1, &100, &STATUS_CANCELLED);
    s.client.update_subscription(&2, &100, &STATUS_EXHAUSTED);

    for id in [1u64, 2] {
        expect_err(
            s.client.try_update_subscription(&id, &100, &STATUS_ACTIVE),
            Error::AlreadyFinal,
        );
        expect_err(
            s.client
                .try_update_subscription(&id, &500, &STATUS_CANCELLED),
            Error::AlreadyFinal,
        );
    }
    assert_eq!(s.stats().total_charged_volume, 200);
}

#[test]
fn unknown_status_codes_are_rejected() {
    let s = Setup::new();
    s.register(1);
    expect_err(
        s.client.try_update_subscription(&1, &0, &4),
        Error::InvalidStatus,
    );
}

#[test]
fn unknown_subscriptions_are_not_found() {
    let s = Setup::new();
    expect_err(
        s.client.try_update_subscription(&9, &0, &STATUS_ACTIVE),
        Error::SubscriptionNotFound,
    );
    expect_err(
        s.client.try_get_subscription_record(&9),
        Error::SubscriptionNotFound,
    );
}

#[test]
fn listings_separate_all_from_active() {
    let s = Setup::new();
    for id in 1..=4u64 {
        s.register(id);
    }
    s.client.update_subscription(&2, &0, &STATUS_PAUSED);
    s.client.update_subscription(&3, &0, &STATUS_CANCELLED);

    assert_eq!(ids(s.client.get_all_subscriptions()), [1, 2, 3, 4]);
    assert_eq!(ids(s.client.get_active_subscriptions()), [1, 4]);
}

#[test]
fn register_plan_counts_each_plan_once() {
    let s = Setup::new();
    s.client.register_plan(&1);
    s.client.register_plan(&2);
    expect_err(s.client.try_register_plan(&1), Error::PlanAlreadyRegistered);
    assert_eq!(s.stats().total_plans, 2);
}

#[test]
fn only_the_plan_contract_can_register_plans() {
    let s = Setup::new();
    let result = s
        .client
        .mock_auths(&[MockAuth {
            address: &s.subs,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "register_plan",
                args: (1u64,).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .try_register_plan(&1);
    assert!(result.is_err());
    assert_eq!(s.stats().total_plans, 0);
}
