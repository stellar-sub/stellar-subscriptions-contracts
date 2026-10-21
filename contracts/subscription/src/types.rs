use soroban_sdk::{contracttype, Address};

/// Lifecycle state of a subscription.
///
/// The discriminants are part of the public interface: the registry and
/// off-chain indexers receive the status as a `u32`, so they must never be
/// renumbered.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum SubStatus {
    /// Chargeable once per interval, within the cap.
    Active = 0,
    /// Paused by the subscriber. No charges until resumed.
    Paused = 1,
    /// Cancelled by the subscriber. Final: no further charges, ever.
    Cancelled = 2,
    /// `total_charged` has reached `total_cap`. Final.
    Exhausted = 3,
}

impl SubStatus {
    /// Cancelled and Exhausted are terminal; nothing moves a subscription out
    /// of them.
    pub fn is_final(self) -> bool {
        matches!(self, SubStatus::Cancelled | SubStatus::Exhausted)
    }
}

/// A subscriber's bounded, revocable authorization for a merchant to pull
/// `amount_per_period` of `token` once every `interval_ledgers`, never
/// exceeding `total_cap` in aggregate.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subscription {
    pub id: u64,
    pub subscriber: Address,
    pub merchant: Address,
    pub token: Address,
    /// Fixed charge taken each interval.
    pub amount_per_period: i128,
    /// Minimum number of ledgers between two successful charges.
    pub interval_ledgers: u32,
    /// Maximum ever chargeable over the life of the subscription.
    pub total_cap: i128,
    /// Running sum of all successful charges. Never exceeds `total_cap`.
    pub total_charged: i128,
    pub start_ledger: u32,
    /// Ledger of the most recent successful charge (0 if never charged).
    pub last_charge_ledger: u32,
    /// Earliest ledger at which the next charge may succeed.
    pub next_charge_ledger: u32,
    pub status: SubStatus,
    /// Plan this subscription was created from, or 0 for an ad-hoc
    /// subscription.
    pub plan_id: u64,
}

#[cfg(test)]
mod golden {
    // Storage-layout golden for `Subscription` and `SubStatus`.
    //
    // A `#[contracttype]` struct is stored on chain as a sorted XDR `ScVal::Map`
    // of (field-name symbol, value) pairs. Adding, removing, renaming or
    // re-typing a field changes that map, which silently breaks reads of every
    // subscription already on chain. This test pins the layout so any such
    // change is a deliberate, visible failure instead of a silent one.
    //
    // The layout is checked three ways, each catching a different drift:
    //   * the field names, in their sorted order — a renamed field is caught
    //     even when its XDR length is unchanged;
    //   * each field's XDR type — a re-typed field (e.g. u64 -> u32) is caught;
    //   * a full encode/decode round-trip — a value that stops surviving the
    //     storage encoding is caught.
    //
    // `SubStatus` is part of the public interface too: the registry and
    // off-chain indexers receive it as a u32, so its discriminants must never
    // be renumbered.
    //
    // To regenerate after an *intended* change, run
    //   cargo test -p subscription golden -- --nocapture
    // then update the `FIELDS` table and the `SubStatus` asserts below, and
    // note the storage-compatibility impact in the PR description. To instead
    // pin the raw XDR bytes, print them with
    //   sc.to_xdr().expect("xdr")
    // and compare against a stored base64 string.

    extern crate std;

    use soroban_sdk::{testutils::Address as _, Address, Env, TryFromVal, xdr::ScVal};

    use super::{SubStatus, Subscription};

    /// The XDR type each field serializes to. Kept as a small enum rather than
    /// a `ScVal` so the expected shape is readable and value-independent.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Kind {
        U32,
        U64,
        I128,
        Address,
    }

    impl Kind {
        fn matches(self, v: &ScVal) -> bool {
            match self {
                Kind::U32 => matches!(v, ScVal::U32(_)),
                Kind::U64 => matches!(v, ScVal::U64(_)),
                Kind::I128 => matches!(v, ScVal::I128(_)),
                Kind::Address => matches!(v, ScVal::Address(_)),
            }
        }
    }

    /// Field name and XDR type, in the sorted order `#[contracttype]` emits.
    const FIELDS: [(&str, Kind); 13] = [
        ("amount_per_period", Kind::I128),
        ("id", Kind::U64),
        ("interval_ledgers", Kind::U32),
        ("last_charge_ledger", Kind::U32),
        ("merchant", Kind::Address),
        ("next_charge_ledger", Kind::U32),
        ("plan_id", Kind::U64),
        ("start_ledger", Kind::U32),
        ("status", Kind::U32),
        ("subscriber", Kind::Address),
        ("token", Kind::Address),
        ("total_cap", Kind::I128),
        ("total_charged", Kind::I128),
    ];

    #[test]
    fn substatus_discriminants_are_the_public_registry_values() {
        assert_eq!(SubStatus::Active as u32, 0);
        assert_eq!(SubStatus::Paused as u32, 1);
        assert_eq!(SubStatus::Cancelled as u32, 2);
        assert_eq!(SubStatus::Exhausted as u32, 3);
    }

    #[test]
    fn subscription_storage_layout_is_stable() {
        let env = Env::default();

        let sub = Subscription {
            id: 1,
            // The values do not matter: only the field names and types pin the
            // layout. Distinct sentinels make a round-trip loss visible.
            subscriber: Address::generate(&env),
            merchant: Address::generate(&env),
            token: Address::generate(&env),
            amount_per_period: 100,
            interval_ledgers: 1_000,
            total_cap: 1_200,
            total_charged: 0,
            start_ledger: 10_000,
            last_charge_ledger: 0,
            next_charge_ledger: 10_000,
            status: SubStatus::Active,
            plan_id: 0,
        };

        let sc: ScVal = (&sub)
            .try_into()
            .expect("Subscription encodes to an ScVal");

        // The value survives a full encode/decode round-trip.
        let back = Subscription::try_from_val(&env, &sc).expect("Subscription decodes");
        assert_eq!(back, sub);

        // The map carries exactly these fields, by name, in this sorted order,
        // each with the XDR type it must keep for old entries to stay readable.
        let map = match &sc {
            ScVal::Map(Some(m)) => m,
            _ => panic!("Subscription must encode to an ScVal::Map"),
        };
        assert_eq!(map.len() as usize, FIELDS.len());
        for (entry, (name, kind)) in map.iter().zip(FIELDS.iter().copied()) {
            let actual = match &entry.key {
                ScVal::Symbol(s) => s.to_utf8_string().expect("field name decodes"),
                _ => panic!("Subscription field key is not a symbol"),
            };
            assert_eq!(actual.as_str(), name, "field name changed or reordered");
            assert!(
                kind.matches(&entry.val),
                "field {name} serializes to the wrong XDR type"
            );
        }
    }
}
