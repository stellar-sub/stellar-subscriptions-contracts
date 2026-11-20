//! Reentrancy boundary: a linked plan contract calling back into the
//! subscription contract from within `get_plan` is rejected by the Soroban
//! host.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, String};

use super::setup::{expect_err, Setup, AMOUNT, CAP, INTERVAL};
use crate::{interfaces::Plan, Error, SubscriptionContractClient};

/// A mock plan contract whose `get_plan` attempts to re-enter the subscription
/// contract while `subscribe` is waiting for `get_plan` to return.
#[contract]
pub struct ReentrantPlan;

#[contractimpl]
impl ReentrantPlan {
    pub fn configure(
        env: Env,
        subscription_contract: Address,
        subscriber: Address,
        merchant: Address,
        token: Address,
        try_call: bool,
    ) {
        env.storage()
            .instance()
            .set(&symbol_short!("subs"), &subscription_contract);
        env.storage()
            .instance()
            .set(&symbol_short!("subr"), &subscriber);
        env.storage()
            .instance()
            .set(&symbol_short!("merch"), &merchant);
        env.storage()
            .instance()
            .set(&symbol_short!("token"), &token);
        env.storage()
            .instance()
            .set(&symbol_short!("try_call"), &try_call);
    }

    pub fn get_plan(env: Env, plan_id: u64) -> Plan {
        let subscription_contract: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("subs"))
            .unwrap();
        let subscriber: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("subr"))
            .unwrap();
        let merchant: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("merch"))
            .unwrap();
        let token: Address = env
            .storage()
            .instance()
            .get(&symbol_short!("token"))
            .unwrap();
        let try_call: bool = env
            .storage()
            .instance()
            .get(&symbol_short!("try_call"))
            .unwrap_or(false);

        let client = SubscriptionContractClient::new(&env, &subscription_contract);

        if try_call {
            // Attempt reentrant `subscribe` and `charge` via `try_*` so we can
            // record that the Soroban host rejected the reentrant invocation,
            // then still return a valid `Plan` to verify outer state safety.
            let nested_sub = client.try_subscribe(
                &subscriber,
                &merchant,
                &token,
                &AMOUNT,
                &INTERVAL,
                &CAP,
                &0,
            );
            env.storage()
                .instance()
                .set(&symbol_short!("sub_err"), &nested_sub.is_err());
        } else {
            // Direct reentrant call traps in the Soroban host, causing the
            // outer `try_get_plan` in `check_plan` to fail.
            client.subscribe(
                &subscriber,
                &merchant,
                &token,
                &AMOUNT,
                &INTERVAL,
                &CAP,
                &0,
            );
        }

        Plan {
            id: plan_id,
            merchant,
            name: String::from_str(&env, "Pro"),
            token,
            amount_per_period: AMOUNT,
            interval_ledgers: INTERVAL,
            default_cap: CAP,
            active: true,
            created_at: env.ledger().sequence(),
            description: None,
        }
    }

    pub fn nested_subscribe_rejected(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&symbol_short!("sub_err"))
            .unwrap_or(false)
    }
}

#[test]
fn plan_contract_reentering_subscription_during_get_plan_is_rejected_by_host() {
    let s = Setup::new();
    let plan = ReentrantPlanClient::new(&s.env, &s.env.register(ReentrantPlan, ()));
    s.client.set_plan_contract(&s.admin, &plan.address);

    // 1. Direct reentrant call inside `get_plan` traps at the Soroban host
    // boundary; `check_plan` catches the failed `try_get_plan` and rejects the
    // subscription with `PlanNotFound`, leaving no record or allowance.
    plan.configure(
        &s.client.address,
        &s.subscriber,
        &s.merchant,
        &s.token.address,
        &false,
    );
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
        Error::PlanNotFound,
    );
    assert!(s.client.get_by_subscriber(&s.subscriber).is_empty());
    assert_eq!(s.allowance(), 0);

    // 2. Even if the malicious plan contract catches the host reentrancy trap
    // via `try_subscribe` and still returns matching plan terms, the reentrant
    // inner call was rejected by the host and only the single outer
    // subscription is created.
    plan.configure(
        &s.client.address,
        &s.subscriber,
        &s.merchant,
        &s.token.address,
        &true,
    );
    let id = s.client.subscribe(
        &s.subscriber,
        &s.merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &1,
    );
    assert!(plan.nested_subscribe_rejected());
    assert_eq!(id, 1);
    assert_eq!(s.client.get_by_subscriber(&s.subscriber).len(), 1);
    assert_eq!(s.allowance(), CAP);
}
