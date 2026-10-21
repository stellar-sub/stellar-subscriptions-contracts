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
    /// Instance: linked plan contract, if any.
    PlanContract,
    /// Instance: linked registry contract, if any.
    Registry,
}

pub fn read_plan_contract(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::PlanContract)
}

pub fn write_plan_contract(env: &Env, plan_contract: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::PlanContract, plan_contract);
}

pub fn read_registry(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Registry)
}

pub fn write_registry(env: &Env, registry: &Address) {
    env.storage().instance().set(&DataKey::Registry, registry);
}

/// Whether any subscription has ever been created.
pub fn has_subscriptions(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::NextId)
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

#[cfg(test)]
mod golden {
    // Storage-layout golden for `DataKey`.
    //
    // A `#[contracttype]` enum is stored on chain as an XDR `ScVal::Vec` whose
    // first element is the variant name as a symbol. Adding, removing,
    // renaming or re-typing a variant changes that encoding and silently
    // breaks reads of every key already on chain. This test pins each variant
    // so any such change is a deliberate, visible failure instead of a silent
    // one.
    //
    // Every variant is named in `covers_all_variants` (no wildcard), so adding
    // or removing one is a compile error. Each variant is also encoded, its
    // leading symbol and payload type are checked, and it is round-tripped, so
    // a rename, re-type or broken encoding is caught.
    //
    // To regenerate after an *intended* change, run
    //   cargo test -p subscription golden -- --nocapture
    // then add the new variant to `covers_all_variants` and the test's case
    // list, and note the storage-compatibility impact in the PR description.

    extern crate std;

    use soroban_sdk::{testutils::Address as _, Address, Env, TryFromVal, xdr::ScVal};

    use super::DataKey;

    /// Touch every variant with no wildcard arm, so a new variant is a compile
    /// error rather than a silently untested key.
    fn covers_all_variants(k: &DataKey) {
        match k {
            DataKey::Admin => {}
            DataKey::NextId => {}
            DataKey::Sub(_) => {}
            DataKey::BySubscriber(_) => {}
            DataKey::ByMerchant(_) => {}
            DataKey::PlanContract => {}
            DataKey::Registry => {}
        }
    }

    #[test]
    fn datakey_storage_layout_is_stable() {
        let env = Env::default();
        let addr = Address::generate(&env);

        let cases: [(&str, DataKey); 7] = [
            ("Admin", DataKey::Admin),
            ("NextId", DataKey::NextId),
            ("Sub", DataKey::Sub(7)),
            ("BySubscriber", DataKey::BySubscriber(addr.clone())),
            ("ByMerchant", DataKey::ByMerchant(addr)),
            ("PlanContract", DataKey::PlanContract),
            ("Registry", DataKey::Registry),
        ];

        for (name, key) in cases {
            covers_all_variants(&key);

            let sc: ScVal = (&key).try_into().expect("DataKey encodes to an ScVal");

            let vec = match &sc {
                ScVal::Vec(Some(v)) => v,
                _ => panic!("{name} must encode to an ScVal::Vec"),
            };

            // The leading symbol is the variant name.
            let actual = match &vec[0] {
                ScVal::Symbol(s) => s.to_utf8_string().expect("variant name decodes"),
                _ => panic!("variant {name} leading element is not a symbol"),
            };
            assert_eq!(actual.as_str(), name, "variant {name} changed its symbol");

            // The payload matches the variant's arity and XDR type, so a
            // re-typed field (e.g. u64 -> u32) is caught.
            match &key {
                DataKey::Admin | DataKey::NextId | DataKey::PlanContract | DataKey::Registry => {
                    assert_eq!(vec.len(), 1, "unit variant {name} gained a payload");
                }
                DataKey::Sub(_) => {
                    assert_eq!(vec.len(), 2, "Sub must carry one value");
                    assert!(
                        matches!(vec[1], ScVal::U64(_)),
                        "Sub payload must be U64"
                    );
                }
                DataKey::BySubscriber(_) | DataKey::ByMerchant(_) => {
                    assert_eq!(vec.len(), 2, "{name} must carry one value");
                    assert!(
                        matches!(vec[1], ScVal::Address(_)),
                        "{name} payload must be an Address"
                    );
                }
            }

            // The value survives a full encode/decode/re-encode round-trip.
            let back = DataKey::try_from_val(&env, &sc).expect("DataKey decodes");
            let back_sc: ScVal = (&back).try_into().expect("DataKey re-encodes");
            assert_eq!(back_sc, sc, "{name} did not round-trip");
        }
    }
}
