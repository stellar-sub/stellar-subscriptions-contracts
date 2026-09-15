use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::{
    errors::Error,
    types::{RegistryStats, SubscriptionRecord, Writers},
};

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
    /// Instance: contracts allowed to write.
    Writers,
    /// Instance: aggregate counters.
    Stats,
    /// Persistent: one subscription's record.
    Record(u64),
    /// Persistent: every registered subscription id, oldest first.
    AllSubscriptions,
    /// Persistent: marks a plan id as counted.
    Plan(u64),
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

pub fn has_writers(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Writers)
}

pub fn write_writers(env: &Env, writers: &Writers) {
    env.storage().instance().set(&DataKey::Writers, writers);
}

pub fn read_writers(env: &Env) -> Result<Writers, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Writers)
        .ok_or(Error::NotConfigured)
}

pub fn read_stats(env: &Env) -> RegistryStats {
    env.storage()
        .instance()
        .get(&DataKey::Stats)
        .unwrap_or_default()
}

pub fn write_stats(env: &Env, stats: &RegistryStats) {
    env.storage().instance().set(&DataKey::Stats, stats);
}

pub fn has_record(env: &Env, id: u64) -> bool {
    env.storage().persistent().has(&DataKey::Record(id))
}

pub fn read_record(env: &Env, id: u64) -> Result<SubscriptionRecord, Error> {
    let key = DataKey::Record(id);
    let record = env
        .storage()
        .persistent()
        .get(&key)
        .ok_or(Error::SubscriptionNotFound)?;
    bump_persistent(env, &key);
    Ok(record)
}

pub fn write_record(env: &Env, record: &SubscriptionRecord) {
    let key = DataKey::Record(record.id);
    env.storage().persistent().set(&key, record);
    bump_persistent(env, &key);
}

pub fn read_all_ids(env: &Env) -> Vec<u64> {
    let key = DataKey::AllSubscriptions;
    match env.storage().persistent().get(&key) {
        Some(ids) => {
            bump_persistent(env, &key);
            ids
        }
        None => Vec::new(env),
    }
}

pub fn push_id(env: &Env, id: u64) {
    let key = DataKey::AllSubscriptions;
    let mut ids = read_all_ids(env);
    ids.push_back(id);
    env.storage().persistent().set(&key, &ids);
    bump_persistent(env, &key);
}

/// Mark a plan as counted. Returns false if it already was.
pub fn mark_plan(env: &Env, plan_id: u64) -> bool {
    let key = DataKey::Plan(plan_id);
    if env.storage().persistent().has(&key) {
        return false;
    }
    env.storage().persistent().set(&key, &true);
    bump_persistent(env, &key);
    true
}
