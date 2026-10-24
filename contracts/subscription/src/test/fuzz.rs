//! Randomised action sequences. Whatever order the subscriber and merchant
//! act in, and however time passes between them, CAP, INTERVAL and
//! REVOCATION must hold after every single step.
//!
//! Two proptest blocks:
//!
//! 1. `invariants_hold_for_any_sequence_of_actions` — the original single-
//!    subscription fuzz, extended to also drive a linked registry and verify
//!    after every step that `active_subscriptions`, `total_subscriptions` and
//!    `total_charged_volume` equal the values derived from contract storage.
//!
//! 2. `two_subscriptions_are_isolated` — fuzz two subscriptions for the same
//!    subscriber and verify that CAP, INTERVAL and REVOCATION hold for each
//!    independently, that cancelling one never affects the other, and that the
//!    token allowance always equals the sum of the two remaining caps whenever
//!    the allowance is fresh (i.e. neither is in a state that would require a
//!    refresh).

extern crate std;

use proptest::prelude::*;
use registry::{RegistryContract, RegistryContractClient};

use super::setup::{Setup, AMOUNT, INTERVAL, START_BALANCE};
use crate::SubStatus;

// ── shared action type ────────────────────────────────────────────────────────

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

// ── helper: attach a real registry to a Setup ────────────────────────────────

/// Register a real registry contract against `s` and return its client.
/// Mirrors the pattern in `integration.rs::linked()`.
fn attach_registry(s: &Setup) -> RegistryContractClient<'static> {
    // The plan contract slot is unused here (no plan_id), so we pass the
    // subscription contract address as a placeholder for the plan writer —
    // it will never call register_plan, so the slot is inert.
    let index =
        RegistryContractClient::new(&s.env, &s.env.register(RegistryContract, ()));
    index.initialize(&s.admin);
    index.set_writers(&s.admin, &s.client.address, &s.client.address);
    s.client.set_registry(&s.admin, &index.address);
    index
}

// ── proptest 1: single subscription + registry mirror ────────────────────────

proptest! {
    #![proptest_config(ProptestConfig {
        cases: fuzz_cases(),
        failure_persistence: Some(Box::new(proptest::test_runner::FileFailurePersistence::WithSource("regressions"))),
        ..ProptestConfig::default()
    })]

    /// CAP, INTERVAL and REVOCATION must hold after every step, and the
    /// registry's aggregate counters must match the values derived from the
    /// subscription record at every step.
    ///
    /// On any mismatch the `prop_assert*` macros print the failing action.
    #[test]
    fn invariants_hold_for_any_sequence_of_actions(
        periods in 1i128..=8,
        // Caps that are not a whole number of periods are included on purpose.
        remainder in 0i128..AMOUNT,
        actions in prop::collection::vec(action(), 1..40),
    ) {
        let s = Setup::new();
        let cap = periods * AMOUNT + remainder;

        // Link the registry before the first subscription so every event is
        // captured from the start.
        let index = attach_registry(&s);

        let id = s.subscribe_with(AMOUNT, INTERVAL, cap);

        let mut cancelled = false;
        let mut last_charge: Option<u32> = None;
        let mut charges: i128 = 0;

        for action in &actions {
            match action {
                Action::Advance(ledgers) => s.advance(*ledgers),
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

            // ── subscription invariants ──────────────────────────────────────
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

            // ── registry mirror invariant ────────────────────────────────────
            // Derive the expected values straight from contract storage and
            // compare against what the registry reports. A mismatch here
            // means the registry drifted from the source of truth.
            let stats = index.get_stats();

            // total_subscriptions: one subscription was created, always 1.
            prop_assert_eq!(
                stats.total_subscriptions, 1u64,
                "registry total_subscriptions wrong after {:?}", action
            );

            // active_subscriptions: 1 iff the subscription is Active.
            let expected_active: u64 = if sub.status == SubStatus::Active { 1 } else { 0 };
            prop_assert_eq!(
                stats.active_subscriptions, expected_active,
                "registry active_subscriptions wrong after {:?}: sub.status={:?}",
                action, sub.status
            );

            // total_charged_volume: equals the running charge total.
            prop_assert_eq!(
                stats.total_charged_volume, sub.total_charged,
                "registry total_charged_volume wrong after {:?}", action
            );
        }
    }
}

// ── proptest 2: two subscriptions for the same subscriber ────────────────────

/// For the two-subscription fuzz we track independent state for each sub.
#[derive(Clone, Debug, Default)]
struct SubState {
    cancelled: bool,
    last_charge: Option<u32>,
    charges: i128,
}

proptest! {
    #![proptest_config(ProptestConfig { cases: fuzz_cases(), ..ProptestConfig::default() })]

    /// Two subscriptions for the same subscriber in the same token must each
    /// satisfy CAP, INTERVAL and REVOCATION independently.
    ///
    /// Additional assertions:
    /// - Cancelling one subscription never changes the other's status or
    ///   `total_charged`.
    /// - The token allowance equals the sum of the two remaining caps
    ///   whenever both are in a non-final state (i.e. the allowance is fresh
    ///   and nothing external could have altered it between steps).
    #[test]
    fn two_subscriptions_are_isolated(
        periods_a in 1i128..=8,
        remainder_a in 0i128..AMOUNT,
        periods_b in 1i128..=8,
        remainder_b in 0i128..AMOUNT,
        // Each action carries a selector: 0 targets sub A, 1 targets sub B.
        actions in prop::collection::vec(
            (0u8..2, action()),
            1..40,
        ),
    ) {
        let s = Setup::new();
        let cap_a = periods_a * AMOUNT + remainder_a;
        let cap_b = periods_b * AMOUNT + remainder_b;

        // Mint enough for both caps; START_BALANCE may not cover both.
        let extra = (cap_a + cap_b).saturating_sub(START_BALANCE).max(0);
        if extra > 0 {
            s.mint(&s.subscriber, extra);
        }

        // Two distinct merchants so charges are independent.
        use soroban_sdk::testutils::Address as _;
        let merchant_b = soroban_sdk::Address::generate(&s.env);
        s.mint(&merchant_b, 1); // give merchant_b an account

        // Subscribe A with the default merchant, B with merchant_b.
        let id_a = s.subscribe_with(AMOUNT, INTERVAL, cap_a);
        let id_b = s.client.subscribe(
            &s.subscriber,
            &merchant_b,
            &s.token.address,
            &AMOUNT,
            &INTERVAL,
            &cap_b,
            &0,
        );

        let mut st_a = SubState::default();
        let mut st_b = SubState::default();

        for (target, action) in &actions {
            // Snapshot the *other* sub before acting so we can detect
            // cross-contamination.
            let other_id   = if *target == 0 { id_b } else { id_a };
            let other_before = s.sub(other_id);

            let (id, merchant, st) = if *target == 0 {
                (id_a, &s.merchant, &mut st_a)
            } else {
                (id_b, &merchant_b, &mut st_b)
            };

            match action {
                Action::Advance(ledgers) => s.advance(*ledgers),
                Action::Charge => {
                    if s.client.try_charge(merchant, &id).is_ok() {
                        prop_assert!(
                            !st.cancelled,
                            "REVOCATION: charge on sub {} succeeded after cancel", id
                        );
                        if let Some(prev) = st.last_charge {
                            prop_assert!(
                                s.ledger() - prev >= INTERVAL,
                                "INTERVAL: sub {} charged at {} and {}", id, prev, s.ledger()
                            );
                        }
                        st.last_charge = Some(s.ledger());
                        st.charges += 1;
                    }
                }
                Action::Pause  => { let _ = s.client.try_pause(&s.subscriber, &id); }
                Action::Resume => { let _ = s.client.try_resume(&s.subscriber, &id); }
                Action::Cancel => {
                    if s.client.try_cancel(&s.subscriber, &id).is_ok() {
                        st.cancelled = true;
                    }
                }
            }

            // ── per-sub invariants ───────────────────────────────────────────
            let sub_a = s.sub(id_a);
            let sub_b = s.sub(id_b);

            // CAP for each.
            prop_assert!(
                sub_a.total_charged <= sub_a.total_cap,
                "CAP violated for sub A after {:?}", action
            );
            prop_assert!(
                sub_b.total_charged <= sub_b.total_cap,
                "CAP violated for sub B after {:?}", action
            );

            // Charges match our counters.
            prop_assert_eq!(sub_a.total_charged, st_a.charges * AMOUNT);
            prop_assert_eq!(sub_b.total_charged, st_b.charges * AMOUNT);

            // REVOCATION.
            if st_a.cancelled {
                prop_assert_eq!(sub_a.status, SubStatus::Cancelled);
            }
            if st_b.cancelled {
                prop_assert_eq!(sub_b.status, SubStatus::Cancelled);
            }

            // ── isolation: the other sub is untouched ────────────────────────
            // Advance actions move the clock for both, but they must not
            // change status or total_charged on a sub that was not acted on.
            // (Advance is the only action that legitimately affects nothing
            // about the other sub's fields — we only skip the field check for
            // Advance because it's a clock move, not a mutation.)
            if !matches!(action, Action::Advance(_)) {
                let other_after = s.sub(other_id);
                prop_assert_eq!(
                    other_after.status, other_before.status,
                    "isolation: status of sub {} changed when acting on the other (action={:?})",
                    other_id, action
                );
                prop_assert_eq!(
                    other_after.total_charged, other_before.total_charged,
                    "isolation: total_charged of sub {} changed when acting on the other (action={:?})",
                    other_id, action
                );
            }

            // ── allowance equals sum of open caps when allowance is fresh ────
            // The allowance is "fresh" (set by the contract, not externally
            // mutated) whenever either subscription was just touched by a
            // subscriber-side action (subscribe, cancel, pause, resume,
            // refresh_allowance). Charge does not call sync; Advance doesn't
            // touch it at all. We check after every subscriber action.
            let is_subscriber_action = matches!(
                action,
                Action::Pause | Action::Resume | Action::Cancel
            );
            if is_subscriber_action {
                let remaining_a = if sub_a.status.is_final() {
                    0
                } else {
                    sub_a.total_cap - sub_a.total_charged
                };
                let remaining_b = if sub_b.status.is_final() {
                    0
                } else {
                    sub_b.total_cap - sub_b.total_charged
                };
                let expected_allowance = remaining_a + remaining_b;
                prop_assert_eq!(
                    s.allowance(), expected_allowance,
                    "allowance mismatch after {:?}: remaining_a={} remaining_b={}",
                    action, remaining_a, remaining_b
                );
            }
        }
    }
}
