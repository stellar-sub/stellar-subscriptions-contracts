//! batch_charge: many charges in one call, all or nothing.

use soroban_sdk::{testutils::Address as _, Address, Vec};

use super::setup::{expect_err, Setup, AMOUNT};
use crate::{Error, MAX_BATCH};

#[test]
fn a_batch_charges_every_subscription() {
    let s = Setup::new();
    let a = s.subscribe();
    let b = s.subscribe();
    let c = s.subscribe();

    s.client
        .batch_charge(&s.merchant, &Vec::from_array(&s.env, [a, b, c]));

    for id in [a, b, c] {
        assert_eq!(s.sub(id).total_charged, AMOUNT);
    }
    assert_eq!(s.balance(&s.merchant), 3 * AMOUNT);
}

#[test]
fn one_failing_id_rolls_back_the_whole_batch() {
    let s = Setup::new();
    let a = s.subscribe();
    let b = s.subscribe();
    // `b` was just charged, so it is not due again and the batch must fail.
    s.client.charge(&s.merchant, &b);

    expect_err(
        s.client
            .try_batch_charge(&s.merchant, &Vec::from_array(&s.env, [a, b])),
        Error::IntervalNotElapsed,
    );
    assert_eq!(s.sub(a).total_charged, 0);
    assert_eq!(s.sub(b).total_charged, AMOUNT);
    assert_eq!(s.balance(&s.merchant), AMOUNT);
}

#[test]
fn another_merchants_subscription_fails_the_batch() {
    let s = Setup::new();
    let a = s.subscribe();
    let stranger = Address::generate(&s.env);

    expect_err(
        s.client
            .try_batch_charge(&stranger, &Vec::from_array(&s.env, [a])),
        Error::NotMerchant,
    );
    assert_eq!(s.sub(a).total_charged, 0);
}

#[test]
fn a_batch_over_the_limit_is_refused() {
    let s = Setup::new();
    let id = s.subscribe();
    let mut ids = Vec::new(&s.env);
    for _ in 0..=MAX_BATCH {
        ids.push_back(id);
    }

    expect_err(
        s.client.try_batch_charge(&s.merchant, &ids),
        Error::BatchTooLarge,
    );
    assert_eq!(s.sub(id).total_charged, 0);
}

#[test]
fn duplicate_id_fails_on_second_occurrence_and_rolls_back() {
    // If the same id appears twice in a batch the first charge succeeds
    // internally, advances next_charge_ledger, and then the second attempt
    // hits IntervalNotElapsed. Because batch_charge is all-or-nothing the
    // entire call is rolled back: the subscription is unchanged and the
    // merchant receives nothing.
    let s = Setup::new();
    let id = s.subscribe();

    expect_err(
        s.client
            .try_batch_charge(&s.merchant, &Vec::from_array(&s.env, [id, id])),
        Error::IntervalNotElapsed,
    );

    // Rolled back: no charge was recorded and no funds moved.
    assert_eq!(s.sub(id).total_charged, 0);
    assert_eq!(s.balance(&s.merchant), 0);
}

#[test]
fn batch_with_exactly_max_ids_succeeds() {
    // MAX_BATCH is the inclusive upper bound: a batch of exactly MAX_BATCH
    // ids must succeed, not be refused.
    let s = Setup::new();
    let mut ids = Vec::new(&s.env);
    for _ in 0..MAX_BATCH {
        ids.push_back(s.subscribe());
    }

    s.client.batch_charge(&s.merchant, &ids);

    // Every subscription was charged exactly once.
    for id in ids.iter() {
        assert_eq!(s.sub(id).total_charged, AMOUNT);
    }
    assert_eq!(s.balance(&s.merchant), MAX_BATCH as i128 * AMOUNT);
}
