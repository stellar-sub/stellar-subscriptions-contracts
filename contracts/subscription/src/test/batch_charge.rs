//! batch_charge: many charges in one call, all or nothing.
//! batch_charge_soft: many charges in one call, collecting per-id results.

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
fn soft_batch_charges_every_subscription() {
    let s = Setup::new();
    let a = s.subscribe();
    let b = s.subscribe();
    let c = s.subscribe();

    let results = s
        .client
        .batch_charge_soft(&s.merchant, &Vec::from_array(&s.env, [a, b, c]));

    assert_eq!(results.len(), 3);
    for result in results.iter() {
        assert_eq!(result, Ok(()));
    }
    for id in [a, b, c] {
        assert_eq!(s.sub(id).total_charged, AMOUNT);
    }
    assert_eq!(s.balance(&s.merchant), 3 * AMOUNT);
}

#[test]
fn soft_batch_reports_every_failure_without_charging() {
    let s = Setup::new();
    let a = s.subscribe();
    let b = s.subscribe();
    // Both were just charged, so neither is due again.
    s.client.charge(&s.merchant, &a);
    s.client.charge(&s.merchant, &b);

    let results = s
        .client
        .batch_charge_soft(&s.merchant, &Vec::from_array(&s.env, [a, b]));

    assert_eq!(results.len(), 2);
    for result in results.iter() {
        assert_eq!(result, Err(Error::IntervalNotElapsed));
    }
    assert_eq!(s.sub(a).total_charged, AMOUNT);
    assert_eq!(s.sub(b).total_charged, AMOUNT);
    assert_eq!(s.balance(&s.merchant), 2 * AMOUNT);
}

#[test]
fn soft_batch_keeps_successes_and_reports_failures() {
    let s = Setup::new();
    let a = s.subscribe();
    let b = s.subscribe();
    let c = s.subscribe();
    // `b` was just charged, so it is not due again.
    s.client.charge(&s.merchant, &b);

    let results = s
        .client
        .batch_charge_soft(&s.merchant, &Vec::from_array(&s.env, [a, b, c]));

    assert_eq!(results.len(), 3);
    assert_eq!(results.get(0), Some(Ok(())));
    assert_eq!(results.get(1), Some(Err(Error::IntervalNotElapsed)));
    assert_eq!(results.get(2), Some(Ok(())));

    // The successful charges persist; the failed one is untouched.
    assert_eq!(s.sub(a).total_charged, AMOUNT);
    assert_eq!(s.sub(b).total_charged, AMOUNT);
    assert_eq!(s.sub(c).total_charged, AMOUNT);
    assert_eq!(s.balance(&s.merchant), 3 * AMOUNT);
}

#[test]
fn soft_batch_reports_wrong_merchant_per_id() {
    let s = Setup::new();
    let a = s.subscribe();
    let stranger = Address::generate(&s.env);

    let results = s
        .client
        .batch_charge_soft(&stranger, &Vec::from_array(&s.env, [a]));

    assert_eq!(results.len(), 1);
    assert_eq!(results.get(0), Some(Err(Error::NotMerchant)));
    assert_eq!(s.sub(a).total_charged, 0);
}

#[test]
fn a_soft_batch_over_the_limit_is_refused() {
    let s = Setup::new();
    let id = s.subscribe();
    let mut ids = Vec::new(&s.env);
    for _ in 0..=MAX_BATCH {
        ids.push_back(id);
    }

    expect_err(
        s.client.try_batch_charge_soft(&s.merchant, &ids),
        Error::BatchTooLarge,
    );
    assert_eq!(s.sub(id).total_charged, 0);
}
