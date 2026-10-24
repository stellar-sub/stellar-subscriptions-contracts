//! due_for_merchant: which of a merchant's subscriptions are due, with paging
//! and the per-call scan cap.

extern crate std;

use soroban_sdk::{testutils::Address as _, Address, Vec};

use super::setup::{Setup, AMOUNT, CAP, INTERVAL};
use crate::MAX_SCAN;

fn ids(list: Vec<u64>) -> std::vec::Vec<u64> {
    list.iter().collect()
}

fn subscribe_for(s: &Setup, merchant: &Address) -> u64 {
    s.client.subscribe(
        &s.subscriber,
        merchant,
        &s.token.address,
        &AMOUNT,
        &INTERVAL,
        &CAP,
        &0,
    )
}

#[test]
fn due_lists_only_chargeable_subscriptions_of_this_merchant() {
    let s = Setup::new();
    let a = s.subscribe();
    let b = s.subscribe();
    let c = s.subscribe();
    let other = Address::generate(&s.env);
    subscribe_for(&s, &other);

    // `a` has just been charged, so it waits out its interval.
    s.client.charge(&s.merchant, &a);
    let (due, next) = s.client.due_for_merchant(&s.merchant, &0, &10);
    assert_eq!(ids(due), std::vec![b, c]);
    assert_eq!(next, None);
}

#[test]
fn cancelled_subscriptions_are_never_due() {
    let s = Setup::new();
    let a = s.subscribe();
    let b = s.subscribe();
    s.client.cancel(&s.subscriber, &a);

    let (due, next) = s.client.due_for_merchant(&s.merchant, &0, &10);
    assert_eq!(ids(due), std::vec![b]);
    assert_eq!(next, None);
}

#[test]
fn paging_resumes_from_the_returned_position_until_the_index_ends() {
    let s = Setup::new();
    let a = s.subscribe();
    let b = s.subscribe();
    let c = s.subscribe();
    s.client.charge(&s.merchant, &a);

    let (first, cursor) = s.client.due_for_merchant(&s.merchant, &0, &1);
    assert_eq!(ids(first), std::vec![b]);
    let cursor = cursor.expect("more of the index remains");

    let (second, cursor) = s.client.due_for_merchant(&s.merchant, &cursor, &1);
    assert_eq!(ids(second), std::vec![c]);
    assert_eq!(cursor, None);
}

#[test]
fn a_call_reads_at_most_max_scan_entries_and_resumes_after_them() {
    let s = Setup::new();
    let mut ids_in_index = std::vec::Vec::new();
    for _ in 0..(MAX_SCAN + 5) {
        ids_in_index.push(s.subscribe());
    }
    // Cancel the first MAX_SCAN, so the first call scans them and finds none due.
    for id in &ids_in_index[..MAX_SCAN as usize] {
        s.client.cancel(&s.subscriber, id);
    }

    let (due, cursor) = s.client.due_for_merchant(&s.merchant, &0, &50);
    assert!(due.is_empty());
    assert_eq!(cursor, Some(MAX_SCAN));

    let (due, cursor) = s.client.due_for_merchant(&s.merchant, &MAX_SCAN, &50);
    assert_eq!(ids(due), ids_in_index[MAX_SCAN as usize..].to_vec());
    assert_eq!(cursor, None);
}
