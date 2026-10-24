extern crate std;

use soroban_sdk::{
    contract, contractimpl, symbol_short,
    testutils::{Address as _, Ledger},
    Address, Env,
};

use crate::{SubscriptionContract, SubscriptionContractClient};

const AMOUNT: i128 = 100;
const INTERVAL: u32 = 1_000;
const CAP: i128 = 1_200;
const START_LEDGER: u32 = 10_000;

#[contract]
struct ReentrantToken;

#[contract]
struct NoopToken;

#[contractimpl]
impl NoopToken {
    pub fn approve(
        _env: Env,
        _from: Address,
        _spender: Address,
        _amount: i128,
        _live_until: u32,
    ) {
    }

    pub fn transfer_from(
        _env: Env,
        _spender: Address,
        _from: Address,
        _to: Address,
        _amount: i128,
    ) {
    }
}

#[contractimpl]
impl ReentrantToken {
    pub fn configure(env: Env, subscription_contract: Address, merchant: Address, id: u64) {
        env.storage()
            .instance()
            .set(&symbol_short!("subs"), &subscription_contract);
        env.storage()
            .instance()
            .set(&symbol_short!("merchant"), &merchant);
        env.storage().instance().set(&symbol_short!("sub_id"), &id);
    }

    pub fn approve(
        _env: Env,
        _from: Address,
        _spender: Address,
        _amount: i128,
        _live_until: u32,
    ) {
    }

    pub fn transfer_from(
        env: Env,
        _spender: Address,
        _from: Address,
        _to: Address,
        _amount: i128,
    ) {
        let subscription_contract = env
            .storage()
            .instance()
            .get::<_, Address>(&symbol_short!("subs"))
            .unwrap();
        let merchant = env
            .storage()
            .instance()
            .get::<_, Address>(&symbol_short!("merchant"))
            .unwrap();
        let id = env
            .storage()
            .instance()
            .get::<_, u64>(&symbol_short!("sub_id"))
            .unwrap();

        let nested = SubscriptionContractClient::new(&env, &subscription_contract)
            .try_charge(&merchant, &id);
        env.storage()
            .instance()
            .set(&symbol_short!("nestfail"), &nested.is_err());
        let calls = env
            .storage()
            .instance()
            .get::<_, u32>(&symbol_short!("calls"))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&symbol_short!("calls"), &(calls + 1));
    }

    pub fn nested_failed(env: Env) -> bool {
        env.storage()
            .instance()
            .get(&symbol_short!("nestfail"))
            .unwrap_or(false)
    }

    pub fn transfer_calls(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&symbol_short!("calls"))
            .unwrap_or(0)
    }
}

#[test]
fn token_callback_cannot_reenter_charge_or_double_charge() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_sequence_number(START_LEDGER);

    let admin = Address::generate(&env);
    let subscriber = Address::generate(&env);
    let merchant = Address::generate(&env);
    let subscription_contract = SubscriptionContractClient::new(
        &env,
        &env.register(SubscriptionContract, ()),
    );
    subscription_contract.initialize(&admin);
    let token = ReentrantTokenClient::new(&env, &env.register(ReentrantToken, ()));
    let noop_token = NoopTokenClient::new(&env, &env.register(NoopToken, ()));
    let outer_id = subscription_contract.subscribe(
        &subscriber,
        &merchant,
        &token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    );
    let nested_id = subscription_contract.subscribe(
        &subscriber,
        &merchant,
        &noop_token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    );
    token.configure(&subscription_contract.address, &merchant, &nested_id);

    subscription_contract.charge(&merchant, &outer_id);

    let outer = subscription_contract.get_subscription(&outer_id);
    let nested = subscription_contract.get_subscription(&nested_id);
    assert!(token.nested_failed());
    assert_eq!(token.transfer_calls(), 1);
    assert_eq!(outer.total_charged, AMOUNT);
    assert_eq!(outer.next_charge_ledger, START_LEDGER + INTERVAL);
    assert_eq!(nested.total_charged, 0);
    assert_eq!(nested.next_charge_ledger, START_LEDGER);
    assert!(subscription_contract.is_chargeable(&nested_id));
}