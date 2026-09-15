//! Randomised action sequences. Whatever order the subscriber and merchant
//! act in, and however time passes between them, CAP, INTERVAL and
//! REVOCATION must hold after every single step.

extern crate std;

use proptest::prelude::*;

use super::setup::{Setup, AMOUNT, INTERVAL, START_BALANCE};
use crate::SubStatus;

#[derive(Clone, Debug)]
enum Action {
    Advance(u32),
    Charge,
    Pause,
    Resume,
    Cancel,
}

fn action() -> impl Strategy<Value = Action> {
    prop_oneof![
        4 => (1u32..=2 * INTERVAL).prop_map(Action::Advance),
        6 => Just(Action::Charge),
        1 => Just(Action::Pause),
        1 => Just(Action::Resume),
        1 => Just(Action::Cancel),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    #[test]
    fn invariants_hold_for_any_sequence_of_actions(
        periods in 1i128..=8,
        // Caps that are not a whole number of periods are included on purpose.
        remainder in 0i128..AMOUNT,
        actions in prop::collection::vec(action(), 1..40),
    ) {
        let s = Setup::new();
        let cap = periods * AMOUNT + remainder;
        let id = s.subscribe_with(AMOUNT, INTERVAL, cap);

        let mut cancelled = false;
        let mut last_charge: Option<u32> = None;
        let mut charges: i128 = 0;

        for action in actions {
            match action {
                Action::Advance(ledgers) => s.advance(ledgers),
                Action::Charge => {
                    if s.client.try_charge(&s.merchant, &id).is_ok() {
                        prop_assert!(!cancelled, "REVOCATION: charge succeeded after cancel");
                        if let Some(previous) = last_charge {
                            prop_assert!(
                                s.ledger() - previous >= INTERVAL,
                                "INTERVAL: charges at {} and {}", previous, s.ledger()
                            );
                        }
                        last_charge = Some(s.ledger());
                        charges += 1;
                    }
                }
                Action::Pause => {
                    let _ = s.client.try_pause(&s.subscriber, &id);
                }
                Action::Resume => {
                    let _ = s.client.try_resume(&s.subscriber, &id);
                }
                Action::Cancel => {
                    if s.client.try_cancel(&s.subscriber, &id).is_ok() {
                        cancelled = true;
                    }
                }
            }

            let sub = s.sub(id);
            prop_assert!(sub.total_charged <= sub.total_cap, "CAP: total_charged over cap");
            prop_assert_eq!(sub.total_charged, charges * AMOUNT);
            prop_assert_eq!(s.balance(&s.merchant), sub.total_charged);
            prop_assert_eq!(s.balance(&s.subscriber), START_BALANCE - sub.total_charged);
            if cancelled {
                prop_assert_eq!(sub.status, SubStatus::Cancelled);
            }
        }
    }
}
