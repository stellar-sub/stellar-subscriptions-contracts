//! Paged and counted merchant listings.

use super::setup::Setup;

#[test]
fn merchant_pages_are_contiguous_and_capped_at_the_limit() {
    let s = Setup::new();
    for _ in 0..5 {
        s.subscribe();
    }
    let full = s.client.get_by_merchant(&s.merchant);
    let first = s.client.get_by_merchant_page(&s.merchant, &0, &2);
    let second = s.client.get_by_merchant_page(&s.merchant, &2, &2);
    let last = s.client.get_by_merchant_page(&s.merchant, &4, &2);
    let past_end = s.client.get_by_merchant_page(&s.merchant, &5, &2);

    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 2);
    assert_eq!(last.len(), 1);
    assert!(past_end.is_empty());

    // The pages run end to end, with no gaps and no repeats.
    assert_eq!(first.get(0).unwrap().id, full.get(0).unwrap().id);
    assert_eq!(first.get(1).unwrap().id, second.get(0).unwrap().id);
    assert_eq!(second.get(1).unwrap().id, last.get(0).unwrap().id);
    assert_eq!(last.get(0).unwrap().id, full.get(4).unwrap().id);

    // A huge limit is capped, and here that still returns all five.
    let capped = s.client.get_by_merchant_page(&s.merchant, &0, &u32::MAX);
    assert_eq!(capped.len(), 5);
}
