//! Tests for `associate_plan`: retroactively linking an ad-hoc subscription
//! to a plan whose terms already match it exactly.

extern crate std;

use plan::{PlanContract, PlanContractClient};
use soroban_sdk::String;

use super::setup::{Setup, AMOUNT, CAP, INTERVAL};
use crate::Error;

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

#[test]
fn associates_an_ad_hoc_subscription_whose_terms_match() {
    let (s, plans) = with_plans();
    let plan_id = create_plan(&s, &plans);
    let sub_id = s.subscribe(); // ad-hoc, plan_id 0, terms == AMOUNT/INTERVAL/CAP

    s.client.associate_plan(&s.subscriber, &sub_id, &plan_id);

    assert_eq!(s.sub(sub_id).plan_id, plan_id);
}

#[test]
fn rejects_mismatched_terms() {
    let (s, plans) = with_plans();
    let plan_id = create_plan(&s, &plans);
    // Different amount than the plan.
    let sub_id = s.subscribe_with(AMOUNT + 1, INTERVAL, CAP);

    let err = s
        .client
        .try_associate_plan(&s.subscriber, &sub_id, &plan_id)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::PlanMismatch);
    assert_eq!(s.sub(sub_id).plan_id, 0);
}

#[test]
fn rejects_inactive_plan() {
    let (s, plans) = with_plans();
    let plan_id = create_plan(&s, &plans);
    let sub_id = s.subscribe();
    plans.deactivate_plan(&s.merchant, &plan_id);

    let err = s
        .client
        .try_associate_plan(&s.subscriber, &sub_id, &plan_id)
        .unwrap_err()
        .unwrap();
    assert_eq!(err, Error::PlanInactive);
    assert_eq!(s.sub(sub_id).plan_id, 0);
}
