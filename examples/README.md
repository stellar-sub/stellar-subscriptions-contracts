# Examples

Runnable CLI scripts that walk through the full subscription lifecycle against the
current Testnet deployment. Each script is small and focused on a single contract
call, and each one **prints what it expects to happen** before (and after) it runs
so you can compare the output with the on-chain result.

## Prerequisites

- A working `stellar` CLI on your `PATH` (see the repo `README.md` for the
  version we pin).
- A funded Testnet identity to act as the merchant/owner.
- A deployed instance of the contracts. The scripts read the contract IDs and
  the demo identities produced by the deploy script, so deploy first:

  ```sh
  DEMO=1 ./scripts/deploy.sh
  ```

  `DEMO=1` creates the demo identities used throughout these examples:

  | Identity        | Role                                   |
  | --------------- | -------------------------------------- |
  | `demo-owner`    | Merchant / plan owner                  |
  | `demo-sub`      | Subscriber                             |
  | `demo-other`    | Unrelated account (negative checks)    |

  The deploy script also writes the deployed contract IDs to the environment
  file the examples source. If you deployed without `DEMO=1`, create the
  identities yourself and export the same names.

- Network settings default to Testnet (`--network testnet`). Override with the
  `NETWORK` environment variable if you are pointing at another network.

## Order to run

Run the scripts in this order — each one depends on state created by the
previous step:

1. **`create-plan.sh`** — `demo-owner` creates a subscription plan (price,
   interval, trial). Prints the expected plan ID and terms.
2. **`subscribe.sh`** — `demo-sub` subscribes to the plan created in step 1.
   Prints the expected subscription state and next charge time.
3. **`charge.sh`** — `demo-owner` charges the subscription for one interval.
   Prints the expected amount moved and the new next-charge time.
4. **`pause.sh`** — `demo-sub` pauses the subscription. Prints the expected
   paused state and that no charge can occur while paused.
5. **`cancel.sh`** — `demo-sub` cancels the subscription. Prints the expected
   terminal state and that further charges are rejected.

Each script is idempotent-friendly: re-running it prints the expected outcome
and the actual result, so you can see how the contract responds to a repeated
call (for example, charging twice in the same interval).

## Notes

- These examples only exercise the public CLI surface; they do not modify the
  contracts. Contract changes must keep CAP, INTERVAL and REVOCATION guarantees
  intact and error codes are append-only (see `CONTRIBUTING.md`).
- If a script fails, check that you sourced the deploy environment and that the
  identities above exist and are funded on Testnet.