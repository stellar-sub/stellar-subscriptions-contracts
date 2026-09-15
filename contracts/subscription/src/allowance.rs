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

/// Allowance expiries are aligned to windows of this many ledgers (about a
/// day).
///
/// Wallets sign the nested `approve` with exactly the arguments seen when
/// simulating, and the transaction lands a few ledgers later. An expiry
/// derived from the exact current ledger would no longer match that
/// signature, so the call would pass simulation and then fail on-chain.
/// Within one window the aligned expiry is identical.
const EXPIRY_WINDOW: u32 = 17_280;

/// The network's maximum expiry, taken from the start of the current window
/// so it is the same at simulation and at submission.
///
/// `max_ttl()` is `max_live_until_ledger - now`, so the result never exceeds
/// the network maximum. It is also never before `now`, which the token
/// requires for a non-zero allowance.
fn aligned_live_until(env: &Env) -> u32 {
    let now = env.ledger().sequence();
    let window_start = now - now % EXPIRY_WINDOW;
    window_start
        .saturating_add(env.storage().max_ttl())
        .max(now)
}

/// Approve this contract to spend exactly [`outstanding`] on the
/// subscriber's behalf, until the aligned maximum expiry.
///
/// Requires the subscriber's authorization for the nested `approve` call, so
/// it may only run inside a subscriber-authorized entry point. Returns the
/// approved amount.
pub fn sync(env: &Env, subscriber: &Address, token: &Address) -> Result<i128, Error> {
    let amount = outstanding(env, subscriber, token)?;
    let live_until = aligned_live_until(env);
    let spender = env.current_contract_address();
    match TokenClient::new(env, token).try_approve(subscriber, &spender, &amount, &live_until) {
        Ok(Ok(())) => Ok(amount),
        _ => Err(Error::ApprovalFailed),
    }
}
