# Design proposal: index subscriptions by plan (#47)

Status: proposal only. No code has been written for this issue, because the
issue asks for maintainer agreement on the design first.

## Problem

Merchants want to see which subscribers took which plan. Today the only way is
to read every subscription of the merchant with `get_by_merchant` and filter
by `plan_id`, which costs one record read per subscription.

## Proposal

1. Add `DataKey::ByPlan(u64)`, a persistent index of subscription ids, written
   at `subscribe` only when `plan_id != 0`. Ad-hoc subscriptions (`plan_id == 0`)
   are not indexed, so they do not add writes.
2. Add `get_by_plan_page(plan_id, start, limit)`, which reads one slice of that
   index and loads only the records in the slice. It uses the same paging helper
   and `MAX_PAGE_LIMIT` as the subscriber and merchant pages.
3. Add `count_by_plan(plan_id)`, which returns the index length, for the same
   reason as `count_by_subscriber`.

## Trade-offs

- Each plan subscription costs one extra index write, the same as the
  subscriber and merchant indexes. Ad-hoc subscriptions pay nothing.
- The index is not backfilled. Subscriptions created before the index exists
  are not listed by plan. A deployment that needs them must migrate, which this
  proposal does not cover.
- Storage keys are a new `DataKey` variant. Under the version rule in
  `version()`, adding an index is a storage-layout change, so `VERSION` must be
  bumped.

## Open questions for maintainers

- Is a backfill needed for existing deployments, or is it acceptable that only
  new plan subscriptions are listed?
- Should `get_by_plan_page` also accept a merchant filter, or is plan id enough
  because plan ids are owned by one merchant?
