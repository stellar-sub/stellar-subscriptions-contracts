extern crate std;

use core::fmt::Debug;

use soroban_sdk::{
    testutils::{Address as _, Events as _, Ledger, MockAuth, MockAuthInvoke},
    Address, Env, Event, IntoVal, String,
};

use crate::{
    Error, Plan, PlanContract, PlanContractClient, PlanCreated, PlanDeactivated, MAX_NAME_LEN,
};

const AMOUNT: i128 = 500;
const INTERVAL: u32 = 17_280;
const CAP: i128 = 12 * AMOUNT;

struct Setup {
    env: Env,
    client: PlanContractClient<'static>,
    admin: Address,
    merchant: Address,
    token: Address,
}

impl Setup {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(1_000);
        let admin = Address::generate(&env);
        let merchant = Address::generate(&env);
        let token = Address::generate(&env);
        let client = PlanContractClient::new(&env, &env.register(PlanContract, ()));
        client.initialize(&admin);
        Setup {
            env,
            client,
            admin,
            merchant,
            token,
        }
    }

    fn name(&self, s: &str) -> String {
        String::from_str(&self.env, s)
    }

    fn create(&self, merchant: &Address, name: &str) -> u64 {
        self.client.create_plan(
            merchant,
            &self.name(name),
            &self.token,
            &AMOUNT,
            &INTERVAL,
            &CAP,
        )
    }

    /// Assert create_plan with these terms fails with exactly `expected`.
    fn expect_create_err(
        &self,
        name: &str,
        amount: i128,
        interval: u32,
        cap: i128,
        expected: Error,
    ) {
        expect_err(
            self.client.try_create_plan(
                &self.merchant,
                &self.name(name),
                &self.token,
                &amount,
                &interval,
                &cap,
            ),
            expected,
        );
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

fn ids(plans: soroban_sdk::Vec<Plan>) -> std::vec::Vec<u64> {
    plans.iter().map(|p| p.id).collect()
}

#[test]
fn initialize_twice_is_rejected() {
    let s = Setup::new();
    expect_err(s.client.try_initialize(&s.admin), Error::AlreadyInitialized);
}

#[test]
fn create_plan_before_initialize_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let client = PlanContractClient::new(&env, &env.register(PlanContract, ()));
    expect_err(
        client.try_create_plan(
            &Address::generate(&env),
            &String::from_str(&env, "Pro"),
            &Address::generate(&env),
            &AMOUNT,
            &INTERVAL,
            &CAP,
        ),
        Error::NotInitialized,
    );
}

#[test]
fn create_plan_stores_every_field() {
    let s = Setup::new();
    let id = s.create(&s.merchant, "Pro monthly");
    assert_eq!(
        s.client.get_plan(&id),
        Plan {
            id,
            merchant: s.merchant.clone(),
            name: s.name("Pro monthly"),
            token: s.token.clone(),
            amount_per_period: AMOUNT,
            interval_ledgers: INTERVAL,
            default_cap: CAP,
            active: true,
            created_at: 1_000,
        }
    );
}

#[test]
fn plan_ids_start_at_one_and_increment() {
    let s = Setup::new();
    assert_eq!(s.create(&s.merchant, "A"), 1);
    assert_eq!(s.create(&s.merchant, "B"), 2);
    assert_eq!(s.create(&Address::generate(&s.env), "C"), 3);
}

#[test]
fn create_plan_requires_the_merchants_signature() {
    let s = Setup::new();
    let impostor = Address::generate(&s.env);
    let name = s.name("Pro");
    let result = s
        .client
        .mock_auths(&[MockAuth {
            address: &impostor,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "create_plan",
                args: (
                    s.merchant.clone(),
                    name.clone(),
                    s.token.clone(),
                    AMOUNT,
                    INTERVAL,
                    CAP,
                )
                    .into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .try_create_plan(&s.merchant, &name, &s.token, &AMOUNT, &INTERVAL, &CAP);
    assert!(result.is_err());
}

#[test]
fn create_plan_rejects_an_empty_or_oversized_name() {
    let s = Setup::new();
    s.expect_create_err("", AMOUNT, INTERVAL, CAP, Error::InvalidName);
    let too_long = "x".repeat(MAX_NAME_LEN as usize + 1);
    s.expect_create_err(&too_long, AMOUNT, INTERVAL, CAP, Error::InvalidName);
}

#[test]
fn create_plan_accepts_a_name_of_exactly_the_maximum_length() {
    let s = Setup::new();
    let longest = "x".repeat(MAX_NAME_LEN as usize);
    let id = s.create(&s.merchant, &longest);
    assert_eq!(s.client.get_plan(&id).name.len(), MAX_NAME_LEN);
}

#[test]
fn create_plan_rejects_a_non_positive_amount() {
    let s = Setup::new();
    for amount in [0, -1] {
        s.expect_create_err("Pro", amount, INTERVAL, CAP, Error::InvalidAmount);
    }
}

#[test]
fn create_plan_rejects_a_zero_interval() {
    let s = Setup::new();
    s.expect_create_err("Pro", AMOUNT, 0, CAP, Error::InvalidInterval);
}

#[test]
fn create_plan_rejects_a_default_cap_below_one_period() {
    let s = Setup::new();
    s.expect_create_err("Pro", AMOUNT, INTERVAL, AMOUNT - 1, Error::InvalidCap);
}

#[test]
fn the_merchant_can_deactivate_their_plan() {
    let s = Setup::new();
    let id = s.create(&s.merchant, "Pro");
    s.client.deactivate_plan(&s.merchant, &id);
    assert!(!s.client.get_plan(&id).active);
}

#[test]
fn another_merchant_cannot_deactivate_a_plan() {
    let s = Setup::new();
    let id = s.create(&s.merchant, "Pro");
    let rival = Address::generate(&s.env);
    expect_err(
        s.client.try_deactivate_plan(&rival, &id),
        Error::NotPlanMerchant,
    );
    assert!(s.client.get_plan(&id).active);
}

#[test]
fn deactivate_requires_the_merchants_signature() {
    let s = Setup::new();
    let id = s.create(&s.merchant, "Pro");
    let result = s
        .client
        .mock_auths(&[MockAuth {
            address: &s.admin,
            invoke: &MockAuthInvoke {
                contract: &s.client.address,
                fn_name: "deactivate_plan",
                args: (s.merchant.clone(), id).into_val(&s.env),
                sub_invokes: &[],
            },
        }])
        .try_deactivate_plan(&s.merchant, &id);
    assert!(result.is_err());
    assert!(s.client.get_plan(&id).active);
}

#[test]
fn deactivating_twice_is_rejected() {
    let s = Setup::new();
    let id = s.create(&s.merchant, "Pro");
    s.client.deactivate_plan(&s.merchant, &id);
    expect_err(
        s.client.try_deactivate_plan(&s.merchant, &id),
        Error::PlanAlreadyInactive,
    );
}

#[test]
fn unknown_plans_are_not_found() {
    let s = Setup::new();
    expect_err(s.client.try_get_plan(&9), Error::PlanNotFound);
    expect_err(
        s.client.try_deactivate_plan(&s.merchant, &9),
        Error::PlanNotFound,
    );
}

#[test]
fn get_plans_by_merchant_lists_only_theirs_including_inactive() {
    let s = Setup::new();
    let other = Address::generate(&s.env);
    let a = s.create(&s.merchant, "A");
    let b = s.create(&other, "B");
    let c = s.create(&s.merchant, "C");
    s.client.deactivate_plan(&s.merchant, &a);

    assert_eq!(ids(s.client.get_plans_by_merchant(&s.merchant)), [a, c]);
    assert_eq!(ids(s.client.get_plans_by_merchant(&other)), [b]);
    assert!(s
        .client
        .get_plans_by_merchant(&Address::generate(&s.env))
        .is_empty());
}

#[test]
fn get_active_plans_excludes_deactivated_plans() {
    let s = Setup::new();
    let other = Address::generate(&s.env);
    let a = s.create(&s.merchant, "A");
    let b = s.create(&other, "B");
    let c = s.create(&s.merchant, "C");
    assert_eq!(ids(s.client.get_active_plans()), [a, b, c]);

    s.client.deactivate_plan(&other, &b);
    assert_eq!(ids(s.client.get_active_plans()), [a, c]);
}

#[test]
fn create_plan_emits_plan_created() {
    let s = Setup::new();
    let id = s.create(&s.merchant, "Pro");
    let expected = PlanCreated {
        plan_id: id,
        merchant: s.merchant.clone(),
        name: s.name("Pro"),
        token: s.token.clone(),
        amount_per_period: AMOUNT,
        interval_ledgers: INTERVAL,
        default_cap: CAP,
    };
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [expected.to_xdr(&s.env, &s.client.address)]
    );
}

#[test]
fn deactivate_plan_emits_plan_deactivated() {
    let s = Setup::new();
    let id = s.create(&s.merchant, "Pro");
    s.client.deactivate_plan(&s.merchant, &id);
    let expected = PlanDeactivated {
        plan_id: id,
        merchant: s.merchant.clone(),
    };
    assert_eq!(
        s.env.events().all().filter_by_contract(&s.client.address),
        [expected.to_xdr(&s.env, &s.client.address)]
    );
}
