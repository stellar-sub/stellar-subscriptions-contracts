//! All three contracts deployed and linked: subscriptions checked against
//! real plans, and a real registry mirroring every lifecycle change.

use plan::{PlanContract, PlanContractClient, PlanCreated};
use registry::{
    RegistryContract, RegistryContractClient, SubscriptionRegistered, SubscriptionUpdated,
    STATUS_ACTIVE, STATUS_CANCELLED, STATUS_EXHAUSTED, STATUS_PAUSED,
};
use soroban_sdk::{
    testutils::{Address as _, Events as _},
    Address, Event, String,
};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL};
use crate::{
    events::{Cancelled, Charged, Subscribed},
    Error,
};

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
            &None,
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
            &0,
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
            &0,
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
            &0,
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
                &0,
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
            &0,
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
            &0,
        ),
        Error::RegistryUpdateFailed,
    );
    assert_eq!(s.allowance(), 0);
}

#[test]
fn plan_mirror_in_subscription_matches_plan_contract_including_description() {
    let l = linked();
    let desc = String::from_str(&l.s.env, "Enterprise plan with priority support");
    let plan_id = l.plans.create_plan(
        &l.s.merchant,
        &String::from_str(&l.s.env, "Enterprise"),
        &l.s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &Some(desc.clone()),
    );

    // Get plan from plan contract
    let plan_from_contract = l.plans.get_plan(&plan_id);

    // Subscribe to verify mirror decodes correctly with description field
    let id = l.subscribe_to(plan_id, CAP);
    let sub = l.s.sub(id);

    // Verify the subscription was created with the plan id
    assert_eq!(sub.plan_id, plan_id);

    // Verify plan contract's plan has the description
    assert_eq!(plan_from_contract.description, Some(desc));
}

#[test]
fn full_three_contract_lifecycle_test() {
    // 1. Deploy all three contracts and link them in the required order.
    let s = Setup::new();
    let plans = PlanContractClient::new(&s.env, &s.env.register(PlanContract, ()));
    let index = RegistryContractClient::new(&s.env, &s.env.register(RegistryContract, ()));

    plans.initialize(&s.admin);
    index.initialize(&s.admin);
    index.set_writers(&s.admin, &s.client.address, &plans.address);
    plans.set_registry(&s.admin, &index.address);
    s.client.set_plan_contract(&s.admin, &plans.address);
    s.client.set_registry(&s.admin, &index.address);

    // Initial registry stats before any plans or subscriptions
    let initial_stats = index.get_stats();
    assert_eq!(initial_stats.total_plans, 0);
    assert_eq!(initial_stats.total_subscriptions, 0);
    assert_eq!(initial_stats.active_subscriptions, 0);
    assert_eq!(initial_stats.total_charged_volume, 0);

    // 2. Create a plan
    let plan_name = String::from_str(&s.env, "Pro");
    let plan_created_at = s.ledger();
    let plan_id = plans.create_plan(
        &s.merchant,
        &plan_name,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &None,
    );

    // Assert PlanCreated event emitted by the plan contract
    assert_eq!(
        s.env.events().all().filter_by_contract(&plans.address),
        [PlanCreated {
            plan_id,
            merchant: s.merchant.clone(),
            name: plan_name,
            token: s.token.address.clone(),
            amount_per_period: AMOUNT,
            interval_ledgers: INTERVAL,
            default_cap: CAP,
            created_at: plan_created_at,
            description: None,
        }
        .to_xdr(&s.env, &plans.address)]
    );

    // Verify registry counted the newly created plan
    let stats_after_plan = index.get_stats();
    assert_eq!(stats_after_plan.total_plans, 1);
    assert_eq!(stats_after_plan.total_subscriptions, 0);
    assert_eq!(stats_after_plan.active_subscriptions, 0);
    assert_eq!(stats_after_plan.total_charged_volume, 0);

    // 3. Subscribe with that plan_id
    let sub_id = s.client.subscribe(
        &s.subscriber,
        &s.merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &plan_id,
        &0,
    );

    // Assert Subscribed event emitted by the subscription contract
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [Subscribed {
            subscription_id: sub_id,
            subscriber: s.subscriber.clone(),
            merchant: s.merchant.clone(),
            token: s.token.address.clone(),
            amount_per_period: AMOUNT,
            interval_ledgers: INTERVAL,
            total_cap: CAP,
            plan_id,
            trial_ledgers: 0,
            expires_at: 0,
        }
        .to_xdr(&s.env, &s.client.address)]
    );

    // Assert SubscriptionRegistered event emitted by the registry contract
    assert_eq!(
        s.env.events().all().filter_by_contract(&index.address),
        [SubscriptionRegistered {
            subscription_id: sub_id,
            subscriber: s.subscriber.clone(),
            merchant: s.merchant.clone(),
        }
        .to_xdr(&s.env, &index.address)]
    );

    // Verify plan subscriber count incremented
    assert_eq!(plans.get_plan(&plan_id).subscriber_count, 1);

    // Verify registry stats after subscription
    let stats_after_sub = index.get_stats();
    assert_eq!(stats_after_sub.total_plans, 1);
    assert_eq!(stats_after_sub.total_subscriptions, 1);
    assert_eq!(stats_after_sub.active_subscriptions, 1);
    assert_eq!(stats_after_sub.total_charged_volume, 0);

    // 4. Charge first time
    let charge1_ledger = s.ledger();
    s.client.charge(&s.merchant, &sub_id);

    // Assert Charged event from subscription contract
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [Charged {
            subscription_id: sub_id,
            merchant: s.merchant.clone(),
            token: s.token.address.clone(),
            amount: AMOUNT,
            total_charged: AMOUNT,
            next_charge_ledger: charge1_ledger + INTERVAL,
            exhausted: false,
        }
        .to_xdr(&s.env, &s.client.address)]
    );

    // Assert SubscriptionUpdated event from registry contract
    assert_eq!(
        s.env.events().all().filter_by_contract(&index.address),
        [SubscriptionUpdated {
            subscription_id: sub_id,
            total_charged: AMOUNT,
            status: STATUS_ACTIVE,
        }
        .to_xdr(&s.env, &index.address)]
    );

    // Verify registry stats after charge 1
    let stats_after_charge1 = index.get_stats();
    assert_eq!(stats_after_charge1.total_plans, 1);
    assert_eq!(stats_after_charge1.total_subscriptions, 1);
    assert_eq!(stats_after_charge1.active_subscriptions, 1);
    assert_eq!(stats_after_charge1.total_charged_volume, AMOUNT);

    // 5. Advance interval and charge second time
    s.advance(INTERVAL);
    let charge2_ledger = s.ledger();
    s.client.charge(&s.merchant, &sub_id);

    // Assert Charged event from subscription contract for charge 2
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [Charged {
            subscription_id: sub_id,
            merchant: s.merchant.clone(),
            token: s.token.address.clone(),
            amount: AMOUNT,
            total_charged: 2 * AMOUNT,
            next_charge_ledger: charge2_ledger + INTERVAL,
            exhausted: false,
        }
        .to_xdr(&s.env, &s.client.address)]
    );

    // Assert SubscriptionUpdated event from registry contract for charge 2
    assert_eq!(
        s.env.events().all().filter_by_contract(&index.address),
        [SubscriptionUpdated {
            subscription_id: sub_id,
            total_charged: 2 * AMOUNT,
            status: STATUS_ACTIVE,
        }
        .to_xdr(&s.env, &index.address)]
    );

    // Verify registry stats after charge 2
    let stats_after_charge2 = index.get_stats();
    assert_eq!(stats_after_charge2.total_plans, 1);
    assert_eq!(stats_after_charge2.total_subscriptions, 1);
    assert_eq!(stats_after_charge2.active_subscriptions, 1);
    assert_eq!(stats_after_charge2.total_charged_volume, 2 * AMOUNT);

    // 6. Cancel
    s.client.cancel(&s.subscriber, &sub_id);

    // Assert Cancelled event from subscription contract
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [Cancelled {
            subscription_id: sub_id,
            subscriber: s.subscriber.clone(),
            total_charged: 2 * AMOUNT,
            unused_cap: CAP - 2 * AMOUNT,
            allowance_updated: true,
            registry_updated: true,
        }
        .to_xdr(&s.env, &s.client.address)]
    );

    // Assert SubscriptionUpdated event from registry contract for cancellation
    assert_eq!(
        s.env.events().all().filter_by_contract(&index.address),
        [SubscriptionUpdated {
            subscription_id: sub_id,
            total_charged: 2 * AMOUNT,
            status: STATUS_CANCELLED,
        }
        .to_xdr(&s.env, &index.address)]
    );

    // 7. Verify RegistryStats and SubscriptionRecord are consistent
    let final_stats = index.get_stats();
    assert_eq!(final_stats.total_plans, 1);
    assert_eq!(final_stats.total_subscriptions, 1);
    assert_eq!(final_stats.active_subscriptions, 0);
    assert_eq!(final_stats.total_charged_volume, 2 * AMOUNT);

    let sub_record = index.get_subscription_record(&sub_id);
    assert_eq!(sub_record.id, sub_id);
    assert_eq!(sub_record.subscriber, s.subscriber);
    assert_eq!(sub_record.merchant, s.merchant);
    assert_eq!(sub_record.token, s.token.address);
    assert_eq!(sub_record.total_charged, 2 * AMOUNT);
    assert_eq!(sub_record.status, STATUS_CANCELLED);

    let sub = s.sub(sub_id);
    assert_eq!(sub.id, sub_record.id);
    assert_eq!(sub.subscriber, sub_record.subscriber);
    assert_eq!(sub.merchant, sub_record.merchant);
    assert_eq!(sub.token, sub_record.token);
    assert_eq!(sub.total_charged, sub_record.total_charged);
    assert_eq!(sub.status, crate::SubStatus::Cancelled);
    assert_eq!(sub.plan_id, plan_id);
    assert_eq!(sub.total_cap, CAP);

    // Registry listings consistency
    assert!(index.get_active_subscriptions().is_empty());
    let all_subs = index.get_all_subscriptions();
    assert_eq!(all_subs.len(), 1);
    assert_eq!(all_subs.get(0).unwrap(), sub_id);

    // Merchant stats consistency
    let merchant_stats = index.get_merchant_stats(s.merchant.clone());
    assert_eq!(merchant_stats.subscriptions, 1);
    assert_eq!(merchant_stats.active_subscriptions, 0);
    assert_eq!(merchant_stats.total_charged_volume, 2 * AMOUNT);

    // Balances and allowance consistency
    assert_eq!(s.balance(&s.merchant), 2 * AMOUNT);
    assert_eq!(s.balance(&s.subscriber), super::setup::START_BALANCE - 2 * AMOUNT);
    assert_eq!(s.allowance(), 0);
}
