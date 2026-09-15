extern crate std;

use core::fmt::Debug;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env,
};

use crate::{Error, SubscriptionContract, SubscriptionContractClient, Subscription};

pub const AMOUNT: i128 = 100;
pub const INTERVAL: u32 = 1_000;
/// Twelve periods.
pub const CAP: i128 = 12 * AMOUNT;
pub const START_BALANCE: i128 = 1_000_000;
pub const START_LEDGER: u32 = 10_000;

pub struct Setup {
    pub env: Env,
    pub client: SubscriptionContractClient<'static>,
    pub token: TokenClient<'static>,
    pub admin: Address,
    pub subscriber: Address,
    pub merchant: Address,
}

/// Everything a charge could touch. A rejected charge must leave all of it
/// unchanged.
#[derive(Debug, PartialEq)]
pub struct Snapshot {
    pub sub: Subscription,
    pub subscriber_balance: i128,
    pub merchant_balance: i128,
    pub allowance: i128,
}

impl Setup {
    pub fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set_sequence_number(START_LEDGER);

        let admin = Address::generate(&env);
        let subscriber = Address::generate(&env);
        let merchant = Address::generate(&env);

        let sac = env.register_stellar_asset_contract_v2(admin.clone());
        let token = TokenClient::new(&env, &sac.address());
        StellarAssetClient::new(&env, &sac.address()).mint(&subscriber, &START_BALANCE);

        let contract_id = env.register(SubscriptionContract, ());
        let client = SubscriptionContractClient::new(&env, &contract_id);
        client.initialize(&admin);

        Setup {
            env,
            client,
            token,
            admin,
            subscriber,
            merchant,
        }
    }

    /// Default subscription: AMOUNT every INTERVAL, capped at CAP.
    pub fn subscribe(&self) -> u64 {
        self.subscribe_with(AMOUNT, INTERVAL, CAP)
    }

    pub fn subscribe_with(&self, amount: i128, interval: u32, cap: i128) -> u64 {
        self.client.subscribe(
            &self.subscriber,
            &self.merchant,
            &self.token.address,
            &amount,
            &interval,
            &cap,
            &0,
        )
    }

    pub fn mint(&self, to: &Address, amount: i128) {
        StellarAssetClient::new(&self.env, &self.token.address).mint(to, &amount);
    }

    pub fn ledger(&self) -> u32 {
        self.env.ledger().sequence()
    }

    pub fn set_ledger(&self, sequence: u32) {
        self.env.ledger().set_sequence_number(sequence);
    }

    pub fn advance(&self, ledgers: u32) {
        self.set_ledger(self.ledger() + ledgers);
    }

    pub fn sub(&self, id: u64) -> Subscription {
        self.client.get_subscription(&id)
    }

    pub fn balance(&self, who: &Address) -> i128 {
        self.token.balance(who)
    }

    /// Allowance the subscriber has granted this contract.
    pub fn allowance(&self) -> i128 {
        self.token.allowance(&self.subscriber, &self.client.address)
    }

    pub fn snapshot(&self, id: u64) -> Snapshot {
        Snapshot {
            sub: self.sub(id),
            subscriber_balance: self.balance(&self.subscriber),
            merchant_balance: self.balance(&self.merchant),
            allowance: self.allowance(),
        }
    }
}

/// Assert that a `try_*` client call failed with exactly `expected`.
pub fn expect_err<T: Debug, C: Debug, I: Debug>(
    result: Result<Result<T, C>, Result<Error, I>>,
    expected: Error,
) {
    match result {
        Err(Ok(actual)) => assert_eq!(actual, expected),
        Err(Err(host)) => panic!("expected {expected:?}, got host error {host:?}"),
        Ok(value) => panic!("expected {expected:?}, but the call succeeded with {value:?}"),
    }
}
