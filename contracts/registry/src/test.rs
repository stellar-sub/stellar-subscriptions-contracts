extern crate std;

use core::fmt::Debug;

use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger as _, MockAuth, MockAuthInvoke},
    Address, Env, Event, IntoVal,
};

use crate::{
    events::{SubscriptionRegistered, SubscriptionUpdated},
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
    token: Address,
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
            token: Address::generate(&env),
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
            .register_subscription(&id, &self.subscriber, &self.merchant, &self.token);
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
            .try_register_subscription(&1, &s.subscriber, &s.merchant, &s.token),
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
            token: s.token.clone(),
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
            .try_register_subscription(&1, &s.subscriber, &s.merchant, &s.token),
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
        .try_register_subscription(&1, &s.subscriber, &s.merchant, &s.token);
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

#[test]
fn version_is_1() {
    let s = Setup::new();
    assert_eq!(s.client.version(), 1);
}

#[test]
fn page_returns_the_requested_window_oldest_first() {
    let s = Setup::new();
    for id in 1..=5 {
        s.register(id);
    }
    let page = s.client.get_all_subscriptions_page(&1, &2);
    assert_eq!(ids(page), std::vec![2, 3]);
}

#[test]
fn page_past_the_end_is_empty_and_short_at_the_tail() {
    let s = Setup::new();
    for id in 1..=3 {
        s.register(id);
    }
    assert_eq!(
        ids(s.client.get_all_subscriptions_page(&2, &10)),
        std::vec![3]
    );
    assert!(s.client.get_all_subscriptions_page(&3, &10).is_empty());
    assert!(s.client.get_all_subscriptions_page(&u32::MAX, &10).is_empty());
}

#[test]
fn pages_cover_the_whole_index_in_order() {
    let s = Setup::new();
    for id in 1..=5 {
        s.register(id);
    }
    let mut collected = std::vec::Vec::new();
    let mut start = 0;
    loop {
        let page = ids(s.client.get_all_subscriptions_page(&start, &2));
        if page.is_empty() {
            break;
        }
        start += page.len() as u32;
        collected.extend(page);
    }
    assert_eq!(collected, ids(s.client.get_all_subscriptions()));
}

#[test]
fn page_size_above_the_maximum_is_rejected() {
    let s = Setup::new();
    s.register(1);
    expect_err(
        s.client
            .try_get_all_subscriptions_page(&0, &(crate::MAX_PAGE_SIZE + 1)),
        Error::PageSizeTooLarge,
    );
    // The maximum itself is allowed.
    assert_eq!(
        ids(s.client.get_all_subscriptions_page(&0, &crate::MAX_PAGE_SIZE)),
        std::vec![1]
    );
}

/// The last invocation emitted exactly `expected` from the registry.
fn assert_only_event<E: Event>(s: &Setup, expected: E) {
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [expected.to_xdr(&s.env, &s.client.address)]
    );
}

#[test]
fn register_emits_subscription_registered() {
    let s = Setup::new();
    s.register(7);
    assert_only_event(
        &s,
        SubscriptionRegistered {
            subscription_id: 7,
            subscriber: s.subscriber.clone(),
            merchant: s.merchant.clone(),
        },
    );
}

#[test]
fn update_emits_subscription_updated_with_new_values() {
    let s = Setup::new();
    s.register(7);
    s.client.update_subscription(&7, &250, &STATUS_PAUSED);
    assert_only_event(
        &s,
        SubscriptionUpdated {
            subscription_id: 7,
            total_charged: 250,
            status: STATUS_PAUSED,
        },
    );
}

#[test]
fn rejected_writes_emit_no_events() {
    let s = Setup::new();
    s.register(7);
    // A decreasing total is refused, so nothing is published.
    s.client.update_subscription(&7, &100, &STATUS_ACTIVE);
    assert!(s.client.try_update_subscription(&7, &50, &STATUS_ACTIVE).is_err());
    assert!(s.env.events().all().filter_by_contract(&s.client.address) == std::vec::Vec::new());
}

#[test]
fn stats_survive_seven_days_without_writes() {
    const DAY_IN_LEDGERS: u32 = 17_280;
    let s = Setup::new();
    s.register(1);
    s.register(2);
    s.client.update_subscription(&2, &300, &STATUS_ACTIVE);
    let before = s.stats();

    let now = s.env.ledger().sequence();
    s.env.ledger().set_sequence_number(now + 7 * DAY_IN_LEDGERS);

    // Stats must live in persistent storage, not in the instance entry whose
    // 7-day TTL is what this change is protecting against.
    s.env.as_contract(&s.client.address, || {
        let key = crate::storage::DataKey::Stats;
        assert!(s.env.storage().persistent().has(&key));
        assert!(!s.env.storage().instance().has(&key));
    });

    assert_eq!(s.stats(), before);
    assert_eq!(s.stats().total_subscriptions, 2);
    assert_eq!(s.stats().active_subscriptions, 2);
    assert_eq!(s.stats().total_charged_volume, 300);
}

#[test]
fn registration_stores_the_payment_token() {
    let s = Setup::new();
    s.register(1);
    assert_eq!(s.client.get_subscription_record(&1).token, s.token);
}

#[test]
fn records_keep_their_own_token_through_updates() {
    let s = Setup::new();
    let other_token = Address::generate(&s.env);
    s.register(1);
    s.client
        .register_subscription(&2, &s.subscriber, &s.merchant, &other_token);

    s.client.update_subscription(&1, &50, &STATUS_ACTIVE);
    s.client.update_subscription(&2, &75, &STATUS_PAUSED);

    assert_eq!(s.client.get_subscription_record(&1).token, s.token);
    assert_eq!(s.client.get_subscription_record(&2).token, other_token);
}
