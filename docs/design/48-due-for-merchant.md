# Design proposal: list a merchant's due subscriptions (#48)

Status: proposal only. No code has been written for this issue, because the
issue asks for maintainer agreement on the design first.

## Problem

The "charge all due" action has to read every subscription of a merchant and
call `is_chargeable` on each. A single view that returns the due ids would let
the client do one read.

## Proposal

Add `due_for_merchant(merchant, start, limit)`:

1. Walk the merchant's index (`ByMerchant`) from position `start`, in order.
2. For each subscription, evaluate the same checks as `is_chargeable`, which
   are status, interval and cap. The scan stops after `limit` ids are found, or
   after a fixed number of records is read, whichever comes first.
3. Return `(ids, next_position)`, where `next_position` is `None` at the end of
   the index. The caller pages through the index until it reaches the end.

`limit` is capped at `MAX_PAGE_LIMIT`, as for the other paged views.

## Trade-offs

- Cost is proportional to records read, not records due. If few subscriptions
  are due, a single call may read many records and return few ids, so the
  scan cap must be kept low enough to stay within the read budget.
- The result is a snapshot. A subscription can change between the read and the
  charge, so the caller must still handle `charge` failing.
- It is a read-only view, so it adds no writes and no storage layout change.

## Open questions for maintainers

- What scan cap is acceptable per call, given the network read limits?
- Should the view return ids only, or the full record for each due
  subscription?
