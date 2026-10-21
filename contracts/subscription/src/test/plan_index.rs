//! Subscriptions indexed by plan: listing, paging and counts.

extern crate std;

use plan::{PlanContract, PlanContractClient};
use soroban_sdk::String;

use super::setup::{Setup, AMOUNT, CAP, INTERVAL};

/// A subscription setup with the plan contract linked, so plan subscriptions
/// can be created.
fn with_plans() -> (Setup, PlanContractClient<'static>) {
    let s = Setup::new();
    let plans = PlanContractClient::new(&s.env, &s.env.register(PlanContract, ()));
    plans.initialize(&s.admin);
    s.client.set_plan_contract(&s.admin, &plans.address);
    (s, plans)
}

fn create_plan(s: &Setup, plans: &PlanContractClient) -> u64 {
    plans.create_plan(
        &s.merchant,
        &String::from_str(&s.env, "Pro"),
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
    )
}

fn subscribe_to(s: &Setup, plan_id: u64) -> u64 {
    s.client.subscribe(
        &s.subscriber,
        &s.merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &plan_id,
    )
}

#[test]
fn a_plan_lists_only_its_own_subscriptions_in_creation_order() {
    let (s, plans) = with_plans();
    let first_plan = create_plan(&s, &plans);
    let second_plan = create_plan(&s, &plans);

    let a = subscribe_to(&s, first_plan);
    s.subscribe(); // ad-hoc, plan id 0
    let b = subscribe_to(&s, second_plan);
    let c = subscribe_to(&s, first_plan);
    let d = subscribe_to(&s, first_plan);

    assert_eq!(s.client.count_by_plan(&first_plan), 3);
    assert_eq!(s.client.count_by_plan(&second_plan), 1);
    let listed: std::vec::Vec<u64> = s
        .client
        .get_by_plan_page(&first_plan, &0, &10)
        .iter()
        .map(|sub| sub.id)
        .collect();
    assert_eq!(listed, std::vec![a, c, d]);
    assert_eq!(s.sub(b).plan_id, second_plan);
}

#[test]
fn plan_pages_run_end_to_end_without_gaps_or_repeats() {
    let (s, plans) = with_plans();
    let plan = create_plan(&s, &plans);
    for _ in 0..5 {
        subscribe_to(&s, plan);
    }
    let full = s.client.get_by_plan_page(&plan, &0, &10);
    let first = s.client.get_by_plan_page(&plan, &0, &2);
    let second = s.client.get_by_plan_page(&plan, &2, &2);
    let last = s.client.get_by_plan_page(&plan, &4, &2);
    let past_end = s.client.get_by_plan_page(&plan, &5, &2);

    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 2);
    assert_eq!(last.len(), 1);
    assert!(past_end.is_empty());
    assert_eq!(first.get(0).unwrap().id, full.get(0).unwrap().id);
    assert_eq!(first.get(1).unwrap().id, full.get(1).unwrap().id);
    assert_eq!(second.get(0).unwrap().id, full.get(2).unwrap().id);
    assert_eq!(second.get(1).unwrap().id, full.get(3).unwrap().id);
    assert_eq!(last.get(0).unwrap().id, full.get(4).unwrap().id);
}

#[test]
fn ad_hoc_subscriptions_are_never_indexed_by_plan() {
    let s = Setup::new();
    s.subscribe();
    s.subscribe();
    assert_eq!(s.client.count_by_plan(&0), 0);
    assert!(s.client.get_by_plan_page(&0, &0, &10).is_empty());
}

#[test]
fn a_plan_with_no_subscriptions_counts_zero() {
    let (s, plans) = with_plans();
    let unused = create_plan(&s, &plans);
    assert_eq!(s.client.count_by_plan(&unused), 0);
    assert!(s.client.get_by_plan_page(&unused, &0, &10).is_empty());
}
