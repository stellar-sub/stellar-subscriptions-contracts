# Design proposal: index subscriptions by plan (#47)

Status: implemented. The `DataKey::ByPlan(u64)` index is written in
`subscribe`, and the public views `get_by_plan`, `get_by_plan_page` and
`count_by_plan` read it.

## Problem

Merchants want to see which subscribers took which plan. Today the only way is
to read every subscription of the merchant with `get_by_merchant` and filter
by `plan_id`, which costs one record read per subscription.

## Shipped design

1. `DataKey::ByPlan(u64)` is a persistent index of subscription ids, written at
   `subscribe` only when `plan_id != 0`. Ad-hoc subscriptions (`plan_id == 0`)
   are not indexed, so they do not add writes and never appear in the plan
   index.
2. `get_by_plan(plan_id)` returns every subscription recorded under the plan
   index, in insertion order.
3. `get_by_plan_page(plan_id, start, limit)` reads one slice of that index and
   loads only the records in the slice. It uses the same paging helper and
   `MAX_PAGE_LIMIT` as the subscriber and merchant pages.
4. `count_by_plan(plan_id)` returns the index length, for the same reason as
   `count_by_subscriber`.

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

## Resolved

- The index is not backfilled; only subscriptions created after the index was
  introduced are listed by plan.
- Plan id alone is enough for the views; no merchant filter was added.
