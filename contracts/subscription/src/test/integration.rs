//! All three contracts deployed and linked: subscriptions checked against
//! real plans, and a real registry mirroring every lifecycle change.

use plan::{PlanContract, PlanContractClient};
use registry::{
    RegistryContract, RegistryContractClient, STATUS_ACTIVE, STATUS_CANCELLED, STATUS_EXHAUSTED,
    STATUS_PAUSED,
};
use soroban_sdk::{testutils::Address as _, Address, String};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL};
use crate::Error;

struct Linked {
    s: Setup,
    plans: PlanContractClient<'static>,
    index: RegistryContractClient<'static>,
}

/// Deploy plan and registry next to the subscription contract and link all
/// three, in the same order scripts/deploy.sh uses.
fn linked() -> Linked {
    let s = Setup::new();
    let plans = PlanContractClient::new(&s.env, &s.env.register(PlanContract, ()));
    let index = RegistryContractClient::new(&s.env, &s.env.register(RegistryContract, ()));

    plans.initialize(&s.admin);
    index.initialize(&s.admin);
    index.set_writers(&s.admin, &s.client.address, &plans.address);
    plans.set_registry(&s.admin, &index.address);
    s.client.set_plan_contract(&s.admin, &plans.address);
    s.client.set_registry(&s.admin, &index.address);

    Linked { s, plans, index }
}

impl Linked {
    fn create_plan(&self) -> u64 {
        self.plans.create_plan(
            &self.s.merchant,
            &String::from_str(&self.s.env, "Pro"),
            &self.s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
        )
    }

    fn subscribe_to(&self, plan_id: u64, cap: i128) -> u64 {
        self.s.client.subscribe(
            &self.s.subscriber,
            &self.s.merchant,
            &self.s.token.address,
            &AMOUNT,
            &INTERVAL,
            &cap,
            &plan_id,
        )
    }
}

#[test]
fn subscribing_to_an_active_plan_with_its_exact_terms_succeeds() {
    let l = linked();
    let plan_id = l.create_plan();
    // The subscriber picks their own cap; only the billing terms must match.
    let id = l.subscribe_to(plan_id, 3 * AMOUNT);
    assert_eq!(l.s.sub(id).plan_id, plan_id);
    assert_eq!(l.s.sub(id).total_cap, 3 * AMOUNT);
}

#[test]
fn subscribing_to_an_unknown_plan_is_rejected() {
    let l = linked();
    expect_err(
        l.s.client.try_subscribe(
            &l.s.subscriber,
            &l.s.merchant,
            &l.s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &99,
        ),
        Error::PlanNotFound,
    );
}

#[test]
fn subscribing_to_a_deactivated_plan_is_rejected() {
    let l = linked();
    let plan_id = l.create_plan();
    l.plans.deactivate_plan(&l.s.merchant, &plan_id);
    expect_err(
        l.s.client.try_subscribe(
            &l.s.subscriber,
            &l.s.merchant,
            &l.s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &plan_id,
        ),
        Error::PlanInactive,
    );
}

#[test]
fn terms_that_differ_from_the_plan_are_rejected() {
    let l = linked();
    let plan_id = l.create_plan();
    let other = Address::generate(&l.s.env);
    let s = &l.s;

    // Wrong merchant, token, amount and interval, one at a time.
    let attempts = [
        (other.clone(), s.token.address.clone(), AMOUNT, INTERVAL),
        (s.merchant.clone(), other.clone(), AMOUNT, INTERVAL),
        (
            s.merchant.clone(),
            s.token.address.clone(),
            AMOUNT + 1,
            INTERVAL,
        ),
        (
            s.merchant.clone(),
            s.token.address.clone(),
            AMOUNT,
            INTERVAL - 1,
        ),
    ];
    for (merchant, token, amount, interval) in attempts {
        expect_err(
            s.client.try_subscribe(
                &s.subscriber,
                &merchant,
                &token,
                &amount,
                &interval,
                &CAP,
                &plan_id,
            ),
            Error::PlanMismatch,
        );
    }
}

#[test]
fn deactivating_a_plan_leaves_existing_subscriptions_chargeable() {
    let l = linked();
    let plan_id = l.create_plan();
    let id = l.subscribe_to(plan_id, CAP);
    l.plans.deactivate_plan(&l.s.merchant, &plan_id);

    // What the subscriber authorized still governs; the plan has no say.
    l.s.client.charge(&l.s.merchant, &id);
    assert_eq!(l.s.balance(&l.s.merchant), AMOUNT);
}

#[test]
fn a_plan_id_without_a_linked_plan_contract_is_rejected() {
    let s = Setup::new();
    expect_err(
        s.client.try_subscribe(
            &s.subscriber,
            &s.merchant,
            &s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &1,
        ),
        Error::PlanContractNotSet,
    );
}

#[test]
fn the_registry_mirrors_the_full_lifecycle() {
    let l = linked();
    let plan_id = l.create_plan();
    let id = l.subscribe_to(plan_id, CAP);

    let stats = l.index.get_stats();
    assert_eq!(
        (
            stats.total_plans,
            stats.total_subscriptions,
            stats.active_subscriptions
        ),
        (1, 1, 1)
    );

    l.s.client.charge(&l.s.merchant, &id);
    assert_eq!(l.index.get_stats().total_charged_volume, AMOUNT);
    assert_eq!(l.index.get_subscription_record(&id).total_charged, AMOUNT);

    l.s.client.pause(&l.s.subscriber, &id);
    assert_eq!(l.index.get_subscription_record(&id).status, STATUS_PAUSED);
    assert_eq!(l.index.get_stats().active_subscriptions, 0);

    l.s.client.resume(&l.s.subscriber, &id);
    assert_eq!(l.index.get_subscription_record(&id).status, STATUS_ACTIVE);
    assert_eq!(l.index.get_stats().active_subscriptions, 1);

    l.s.client.cancel(&l.s.subscriber, &id);
    assert_eq!(
        l.index.get_subscription_record(&id).status,
        STATUS_CANCELLED
    );
    assert_eq!(l.index.get_stats().active_subscriptions, 0);
    assert!(l.index.get_active_subscriptions().is_empty());
    assert_eq!(l.index.get_all_subscriptions().len(), 1);
}

#[test]
fn the_registry_records_exhaustion() {
    let l = linked();
    let id = l.subscribe_to(0, AMOUNT);
    l.s.client.charge(&l.s.merchant, &id);

    let record = l.index.get_subscription_record(&id);
    assert_eq!(record.status, STATUS_EXHAUSTED);
    assert_eq!(record.total_charged, AMOUNT);
    assert_eq!(l.index.get_stats().active_subscriptions, 0);
}

#[test]
fn rejected_charges_never_reach_the_registry() {
    let l = linked();
    let id = l.subscribe_to(0, CAP);
    l.s.client.charge(&l.s.merchant, &id);
    expect_err(
        l.s.client.try_charge(&l.s.merchant, &id),
        Error::IntervalNotElapsed,
    );
    l.s.client.cancel(&l.s.subscriber, &id);
    l.s.advance(INTERVAL);
    expect_err(
        l.s.client.try_charge(&l.s.merchant, &id),
        Error::SubscriptionCancelled,
    );
    assert_eq!(l.index.get_stats().total_charged_volume, AMOUNT);
}

#[test]
fn links_are_set_once_and_only_by_the_admin() {
    let l = linked();
    let other = Address::generate(&l.s.env);
    expect_err(
        l.s.client.try_set_registry(&l.s.admin, &other),
        Error::AlreadyConfigured,
    );
    expect_err(
        l.s.client.try_set_plan_contract(&l.s.admin, &other),
        Error::AlreadyConfigured,
    );
    assert!(matches!(
        l.plans.try_set_registry(&l.s.admin, &other),
        Err(Ok(plan::Error::AlreadyConfigured))
    ));

    // Addresses belong to one Env, so the fresh setup needs its own.
    let fresh = Setup::new();
    let registry = Address::generate(&fresh.env);
    expect_err(
        fresh.client.try_set_registry(&fresh.merchant, &registry),
        Error::Unauthorized,
    );
}

#[test]
fn links_cannot_be_added_after_the_first_subscription() {
    let s = Setup::new();
    s.subscribe();
    let other = Address::generate(&s.env);
    expect_err(
        s.client.try_set_registry(&s.admin, &other),
        Error::AlreadyConfigured,
    );
    expect_err(
        s.client.try_set_plan_contract(&s.admin, &other),
        Error::AlreadyConfigured,
    );
}

#[test]
fn a_misconfigured_registry_blocks_subscribe_instead_of_losing_the_record() {
    let s = Setup::new();
    // Initialized, but never told which contracts may write to it.
    let index = RegistryContractClient::new(&s.env, &s.env.register(RegistryContract, ()));
    index.initialize(&s.admin);
    s.client.set_registry(&s.admin, &index.address);

    expect_err(
        s.client.try_subscribe(
            &s.subscriber,
            &s.merchant,
            &s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &0,
        ),
        Error::RegistryUpdateFailed,
    );
    assert_eq!(s.allowance(), 0);
}
