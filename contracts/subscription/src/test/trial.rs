//! is_chargeable must be false for every ledger during the trial period and
//! true exactly at trial end, across several trial lengths.

use super::setup::{Setup, AMOUNT, CAP, INTERVAL};

fn assert_trial_is_respected(trial_ledgers: u32) {
    let s = Setup::new();
    let start = s.ledger();
    let id = s.client.subscribe(
        &s.subscriber,
        &s.merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
        &trial_ledgers,
    );

    for offset in 0..trial_ledgers {
        s.set_ledger(start + offset);
        assert!(
            !s.client.is_chargeable(&id),
            "expected not chargeable at ledger {} (trial_ledgers={trial_ledgers})",
            start + offset
        );
    }

    s.set_ledger(start + trial_ledgers);
    assert!(
        s.client.is_chargeable(&id),
        "expected chargeable at trial end (trial_ledgers={trial_ledgers})"
    );
}

#[test]
fn trial_period_blocks_charging_until_it_ends_short() {
    assert_trial_is_respected(1);
}

#[test]
fn trial_period_blocks_charging_until_it_ends_medium() {
    assert_trial_is_respected(50);
}

#[test]
fn trial_period_blocks_charging_until_it_ends_long() {
    assert_trial_is_respected(500);
}

#[test]
fn no_trial_is_immediately_chargeable() {
    assert_trial_is_respected(0);
}
