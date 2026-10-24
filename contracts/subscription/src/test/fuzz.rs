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
    /// The subscriber calls `refresh_allowance`.
    RefreshAllowance,
    /// The subscriber sets the token allowance directly, to any amount.
    SetAllowance(i128),
}

fn action() -> impl Strategy<Value = Action> {
    prop_oneof![
        4 => (1u32..=2 * INTERVAL).prop_map(Action::Advance),
        6 => Just(Action::Charge),
        1 => Just(Action::Pause),
        1 => Just(Action::Resume),
        1 => Just(Action::Cancel),
        2 => Just(Action::RefreshAllowance),
        2 => (0i128..=2_000).prop_map(Action::SetAllowance),
    ]
}

/// Cases per run: 48 locally, overridable with `PROPTEST_CASES` so CI can fuzz
/// far deeper than a developer wants to wait for.
fn fuzz_cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(48)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: fuzz_cases(), ..ProptestConfig::default() })]

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
                    // The read-only check must predict the outcome of a real charge.
                    let chargeable = s.client.is_chargeable(&id);
                    let charged = s.client.try_charge(&s.merchant, &id).is_ok();
                    prop_assert_eq!(
                        chargeable,
                        charged,
                        "is_chargeable disagreed with try_charge at ledger {}",
                        s.ledger()
                    );
                    if charged {
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
                Action::RefreshAllowance => {
                    if s
                        .client
                        .try_refresh_allowance(&s.subscriber, &s.token.address)
                        .is_ok()
                    {
                        // A refresh sets the allowance to exactly what is still
                        // chargeable, so it can never raise a cap.
                        prop_assert_eq!(s.allowance(), s.client.remaining_cap(&id));
                    }
                }
                Action::SetAllowance(amount) => {
                    // Direct approvals are not checked by the contract. They must
                    // not affect CAP, INTERVAL or REVOCATION, which the checks
                    // below still assert.
                    s.token.approve(
                        &s.subscriber,
                        &s.client.address,
                        &amount,
                        &(s.ledger() + 4 * INTERVAL),
                    );
                }
            }

            let sub = s.sub(id);
            prop_assert!(sub.total_charged <= sub.total_cap, "CAP: total_charged over cap");
            let remaining = s.client.remaining_cap(&id);
            if sub.status.is_final() {
                prop_assert_eq!(remaining, 0, "CAP: final subscription has remaining cap");
            } else {
                prop_assert_eq!(
                    remaining + sub.total_charged,
                    sub.total_cap,
                    "CAP: remaining cap plus charged differs from total cap"
                );
            }
            prop_assert_eq!(sub.total_charged, charges * AMOUNT);
            prop_assert_eq!(s.balance(&s.merchant), sub.total_charged);
            prop_assert_eq!(s.balance(&s.subscriber), START_BALANCE - sub.total_charged);
            if cancelled {
                prop_assert_eq!(sub.status, SubStatus::Cancelled);
            }
        }
    }
}
