//! Subscriptions to different merchants are independent. Charging one
//! merchant must never change another merchant's subscription, balance or
//! schedule, whatever order the charges come in.

extern crate std;

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address};

use super::setup::{Setup, AMOUNT, CAP, INTERVAL};

#[derive(Clone, Debug)]
enum Step {
    Advance(u32),
    ChargeFirst,
    ChargeSecond,
}

fn step() -> impl Strategy<Value = Step> {
    prop_oneof![
        3 => (1u32..=2 * INTERVAL).prop_map(Step::Advance),
        2 => Just(Step::ChargeFirst),
        2 => Just(Step::ChargeSecond),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    #[test]
    fn one_merchants_charges_never_touch_another_merchants_subscription(
        steps in prop::collection::vec(step(), 1..30),
    ) {
        let s = Setup::new();
        let second_merchant = Address::generate(&s.env);
        let first = s.subscribe();
        let second = s.client.subscribe(
            &s.subscriber,
            &second_merchant,
            &s.token.address,
            &AMOUNT,
            &INTERVAL,
            &CAP,
            &0,
        );

        for step in steps {
            match step {
                Step::Advance(ledgers) => s.advance(ledgers),
                Step::ChargeFirst => {
                    let untouched = s.sub(second);
                    let untouched_merchant = s.balance(&second_merchant);
                    let _ = s.client.try_charge(&s.merchant, &first);
                    prop_assert_eq!(s.sub(second), untouched);
                    prop_assert_eq!(s.balance(&second_merchant), untouched_merchant);
                }
                Step::ChargeSecond => {
                    let untouched = s.sub(first);
                    let untouched_merchant = s.balance(&s.merchant);
                    let _ = s.client.try_charge(&second_merchant, &second);
                    prop_assert_eq!(s.sub(first), untouched);
                    prop_assert_eq!(s.balance(&s.merchant), untouched_merchant);
                }
            }
        }
    }

    #[test]
    fn a_merchant_cannot_charge_another_merchants_subscription(
        ledgers in 1u32..=2 * INTERVAL,
    ) {
        let s = Setup::new();
        let second_merchant = Address::generate(&s.env);
        let first = s.subscribe();
        s.advance(ledgers);

        let before = s.sub(first);
        prop_assert!(s.client.try_charge(&second_merchant, &first).is_err());
        prop_assert_eq!(s.sub(first), before);
    }
}
