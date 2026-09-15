//! Token allowance bookkeeping.
//!
//! A Soroban token `transfer` needs the payer's signature at call time,
//! which a pull payment cannot have. Instead the subscriber approves this
//! contract as a spender when subscribing, and `charge` pulls with
//! `transfer_from`.
//!
//! The allowance is a defence-in-depth ceiling, not the enforcement
//! mechanism: CAP, INTERVAL and REVOCATION are enforced by the contract's own
//! checks. It is kept equal to the sum of cap still chargeable across the
//! subscriber's non-final subscriptions in that token, recomputed from
//! contract state rather than adjusted incrementally, so a stale or expired
//! allowance is repaired the next time the subscriber touches it.

use soroban_sdk::{token::TokenClient, Address, Env};

use crate::{
    errors::Error,
    storage::{self, DataKey},
};

/// Cap still chargeable across `subscriber`'s non-final subscriptions in
/// `token`.
pub fn outstanding(env: &Env, subscriber: &Address, token: &Address) -> Result<i128, Error> {
    let ids = storage::read_index(env, &DataKey::BySubscriber(subscriber.clone()));
    let mut total: i128 = 0;
    for id in ids.iter() {
        let sub = storage::read_sub(env, id)?;
        if sub.token != *token || sub.status.is_final() {
            continue;
        }
        let remaining = sub
            .total_cap
            .checked_sub(sub.total_charged)
            .ok_or(Error::Overflow)?;
        total = total.checked_add(remaining).ok_or(Error::Overflow)?;
    }
    Ok(total)
}

/// Approve this contract to spend exactly [`outstanding`] on the
/// subscriber's behalf, valid for as long as the network allows.
///
/// Requires the subscriber's authorization for the nested `approve` call, so
/// it may only run inside a subscriber-authorized entry point. Returns the
/// approved amount.
pub fn sync(env: &Env, subscriber: &Address, token: &Address) -> Result<i128, Error> {
    let amount = outstanding(env, subscriber, token)?;
    let live_until = env.ledger().max_live_until_ledger();
    let spender = env.current_contract_address();
    match TokenClient::new(env, token).try_approve(subscriber, &spender, &amount, &live_until) {
        Ok(Ok(())) => Ok(amount),
        _ => Err(Error::ApprovalFailed),
    }
}
