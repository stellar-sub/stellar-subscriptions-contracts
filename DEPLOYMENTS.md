# Deployments

## Stellar Testnet — current

| | |
|---|---|
| Network | Testnet (`Test SDF Network ; September 2015`), protocol 28 |
| Deployed | 2026-09-15 |
| Built from | commit `a746709` |
| Toolchain | soroban-sdk 27.0.6, stellar CLI 26.1.0 |
| Admin / deployer | `GBQHBOJHJ3SPU5TYNOABLKIXOZQFMN25ZNALA3QC46Q3I4IN5Q2KXZXI` |

| Contract | Contract id | Wasm hash |
|---|---|---|
| Subscription | [`CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM`](https://stellar.expert/explorer/testnet/contract/CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM) | `9eb0f64dec5505ceff5d3c88a1f4fa1a7d8f0f82d49a4aac5d6d6ce187c1c1f2` |
| Plan | [`CDKFRKPA2EIYMW2GX2FY4ZFQ2VAFS5UZD5BHAZLCIGRA7N24N2DFAYN3`](https://stellar.expert/explorer/testnet/contract/CDKFRKPA2EIYMW2GX2FY4ZFQ2VAFS5UZD5BHAZLCIGRA7N24N2DFAYN3) | `3d6e2255348920378f386fe4a4cbd3e2684edc23b3c4ee5b3992416fa092486f` |
| Registry | [`CB7HBXFNBLUSBY3SMIYUPVTE3JZTISYRMZB7XXPUC3ZPLIDNPTKUX645`](https://stellar.expert/explorer/testnet/contract/CB7HBXFNBLUSBY3SMIYUPVTE3JZTISYRMZB7XXPUC3ZPLIDNPTKUX645) | `ef63512273a556fdee358b3b8cc180615868fb8705ae95ee2017b5dfd8b3224a` |

### Links

All links were set by `scripts/deploy.sh` before any plan or subscription
existed, and cannot be changed:

- subscription → plan contract: `CDKFRKPA…AYN3`
- subscription → registry: `CB7HBXFN…X645`
- plan → registry: `CB7HBXFN…X645`
- registry writers: subscription `CAW5H4BH…KXMM`, plan `CDKFRKPA…AYN3`

### Live demo records

Created by `DEMO=1 ./scripts/deploy.sh` with native XLM
(`CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC`):

| Record | Value |
|---|---|
| Merchant | `GCLAL5UFF47MT7JMJHSS4BWXMMKJSTGYE62HJ5X77N45UKLF7BIDKKTU` |
| Subscriber | `GBKAVFZ5GCZMMIV3ZKZL5USKFYT5DBBZTY4P5V5J75ANHWTX7GASETK4` |
| Plan 1 | "Demo hourly": 1 XLM (10,000,000 stroops) every 720 ledgers, default cap 12 XLM |
| Subscription 1 | Plan 1, cap 12 XLM, started at ledger 4,692,992 |
| Charge | 1 XLM at ledger 4,692,994; next charge not before ledger 4,693,714 |
| INTERVAL check | A second charge in the same interval was submitted and rejected on-chain |
| Registry stats | 1 plan, 1 subscription, 1 active, volume 10,000,000 |

### Verify

```bash
SUB=CAW5H4BH5VXWR23DFUVMY45TL7JSMTDZTH3E7LMMXW5LZ7FMDL33KXMM
REG=CB7HBXFNBLUSBY3SMIYUPVTE3JZTISYRMZB7XXPUC3ZPLIDNPTKUX645

stellar contract invoke --id $SUB --network testnet --source-account <any-identity> -- get_subscription --subscription_id 1
stellar contract invoke --id $SUB --network testnet --source-account <any-identity> -- remaining_cap --subscription_id 1
stellar contract invoke --id $REG --network testnet --source-account <any-identity> -- get_stats
```

To confirm the deployed code matches this repository, check out `a746709`,
run `stellar contract build`, and compare the wasm hashes above.

## Superseded Testnet deployments — do not use

Both were built before commit `a746709` and contain the allowance-expiry bug
it fixes: `subscribe` passes simulation but fails on submission once the
ledger advances. They were abandoned, not upgraded.

| Attempt | Subscription | Plan | Registry | What happened |
|---|---|---|---|---|
| 1 | `CCWJ2YLJPBE2R6O2TTXRQULKSXDG3TEU4L2SLGG7LM2LIUPIXACQEMDI` | `CBUB2AXHJ3GWRISMTJPBHAV7ES4HQWYVCKW3DGUK3T3ZISC6D7NJYJR6` | `CDHBF5Q3C4NAYFDFIIMBOJTMSAOIZLFLZYV7ACCLTNVA2DH2SHVXP5XZ` | Deployed and linked; demo stopped early on a deploy-script bug (fixed in `08ef110`). |
| 2 | `CDFELABTDUO6VZJTUBPTY6AJDICM6OTGIX4OAPHL6LZMNOYFKNKWLP4R` | `CD34CSKPOGR7DZKDVGE54WQSLKPYECR6EFUTNGSNUUSATXXWJUCDQS3K` | `CABZHB6R6TXGIGFXEHWOIUK7WYWRWZNQKU7ECN2GMB6LIFXQHZRKZZWC` | Demo plan created, then `subscribe` trapped, which exposed the expiry bug. |

## Mainnet

Not deployed.
