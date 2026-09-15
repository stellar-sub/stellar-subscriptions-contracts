use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::{errors::Error, types::Subscription};

const DAY_IN_LEDGERS: u32 = 17_280;

const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = INSTANCE_BUMP_AMOUNT - DAY_IN_LEDGERS;

const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const PERSISTENT_LIFETIME_THRESHOLD: u32 = PERSISTENT_BUMP_AMOUNT - DAY_IN_LEDGERS;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// Instance: configured admin.
    Admin,
    /// Instance: id the next subscription will receive. Ids start at 1.
    NextId,
    /// Persistent: a subscription record.
    Sub(u64),
    /// Persistent: ids of every subscription created by a subscriber.
    BySubscriber(Address),
    /// Persistent: ids of every subscription payable to a merchant.
    ByMerchant(Address),
}

pub fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

fn bump_persistent(env: &Env, key: &DataKey) {
    env.storage().persistent().extend_ttl(
        key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn write_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn read_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)
}

/// Reserve the next subscription id.
pub fn take_next_id(env: &Env) -> Result<u64, Error> {
    let id: u64 = env.storage().instance().get(&DataKey::NextId).unwrap_or(1);
    let next = id.checked_add(1).ok_or(Error::Overflow)?;
    env.storage().instance().set(&DataKey::NextId, &next);
    Ok(id)
}

pub fn read_sub(env: &Env, id: u64) -> Result<Subscription, Error> {
    let key = DataKey::Sub(id);
    let sub = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::SubscriptionNotFound)?;
    bump_persistent(env, &key);
    Ok(sub)
}

pub fn write_sub(env: &Env, sub: &Subscription) {
    let key = DataKey::Sub(sub.id);
    env.storage().persistent().set(&key, sub);
    bump_persistent(env, &key);
}

pub fn read_index(env: &Env, key: &DataKey) -> Vec<u64> {
    match env.storage().persistent().get(key) {
        Some(ids) => {
            bump_persistent(env, key);
            ids
        }
        None => Vec::new(env),
    }
}

pub fn push_index(env: &Env, key: DataKey, id: u64) {
    let mut ids = read_index(env, &key);
    ids.push_back(id);
    env.storage().persistent().set(&key, &ids);
    bump_persistent(env, &key);
}
