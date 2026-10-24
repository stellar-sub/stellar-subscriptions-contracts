# Stellar Subscriptions Contracts — Storage Layout Reference

This document describes the complete storage layout for all three contracts in the Stellar Subscriptions workspace. It is intended to prevent key collisions and serve as a reference for upgrades and debugging.

## Overview

| Contract | Instance Keys | Persistent Keys |
|----------|--------------|-----------------|
| **registry** | Admin, Writers, Stats | Record(u64), AllSubscriptions, Plan(u64) |
| **subscription** | Admin, NextId, PlanContract, Registry | Sub(u64), BySubscriber(Address), ByMerchant(Address) |
| **plan** | Admin, NextId, Registry | Plan(u64), ByMerchant(Address), AllPlans |

---

## Registry Contract (`contracts/registry`)

### Instance Storage

| Key | Type | Description |
|-----|------|-------------|
| `Admin` | `Address` | Contract administrator |
| `Writers` | `Writers` | Contracts authorized to write (plan, subscription) |
| `Stats` | `RegistryStats` | Aggregate counters (total subscriptions, plans, etc.) |

### Persistent Storage

| Key | Type | Description |
|-----|------|-------------|
| `Record(u64)` | `SubscriptionRecord` | Individual subscription record by ID |
| `AllSubscriptions` | `Vec<u64>` | Ordered list of all subscription IDs (oldest first) |
| `Plan(u64)` | `bool` | Marks a plan ID as registered/counted (deduplication) |

### Collision Analysis

- **No internal collisions**: Each variant has a distinct discriminant
- **Cross-contract**: Uses different contract address, so no collision possible
- **Plan(u64) vs Record(u64)**: Different contracts, different prefixes — safe

---

## Subscription Contract (`contracts/subscription`)

### Instance Storage

| Key | Type | Description |
|-----|------|-------------|
| `Admin` | `Address` | Contract administrator |
| `NextId` | `u64` | Next subscription ID to assign (starts at 1) |
| `PlanContract` | `Option<Address>` | Linked plan contract address |
| `Registry` | `Option<Address>` | Linked registry contract address |

### Persistent Storage

| Key | Type | Description |
|-----|------|-------------|
| `Sub(u64)` | `Subscription` | Full subscription record by ID |
| `BySubscriber(Address)` | `Vec<u64>` | Subscription IDs created by this subscriber |
| `ByMerchant(Address)` | `Vec<u64>` | Subscription IDs payable to this merchant |

### Collision Analysis

- **No internal collisions**: Distinct discriminants for each variant
- **Sub(u64) vs Plan(u64)**: Different contracts — safe
- **BySubscriber/ByMerchant**: Use `Address` as key — distinct from u64 keys

---

## Plan Contract (`contracts/plan`)

### Instance Storage

| Key | Type | Description |
|-----|------|-------------|
| `Admin` | `Address` | Contract administrator |
| `NextId` | `u64` | Next plan ID to assign (starts at 1) |
| `Registry` | `Option<Address>` | Linked registry contract address |

### Persistent Storage

| Key | Type | Description |
|-----|------|-------------|
| `Plan(u64)` | `Plan` | Plan record by ID |
| `ByMerchant(Address)` | `Vec<u64>` | Plan IDs created by this merchant |
| `AllPlans` | `Vec<u64>` | Ordered list of all plan IDs (oldest first) |

### Collision Analysis

- **No internal collisions**: Distinct discriminants
- **Plan(u64) vs Record(u64) vs Sub(u64)**: Different contracts — safe
- **ByMerchant(Address)**: Uses Address — distinct from u64 keys

---

## Cross-Contract Collision Safety

### Instance vs Persistent
- Instance and persistent storage are separate namespaces in Soroban
- A key in instance storage cannot collide with a key in persistent storage

### Contract Isolation
Each contract has its own storage namespace (keyed by contract address). Keys from different contracts **cannot** collide even if they have identical discriminants and values.

### Discriminant Uniqueness Within Contract
All `DataKey` enums use Rust's default discriminant assignment (0, 1, 2...). Within each contract:
- Registry: 6 variants (Admin=0, Writers=1, Stats=2, Record=3, AllSubscriptions=4, Plan=5)
- Subscription: 7 variants (Admin=0, NextId=1, Sub=2, BySubscriber=3, ByMerchant=4, PlanContract=5, Registry=6)
- Plan: 6 variants (Admin=0, NextId=1, Plan=2, ByMerchant=3, AllPlans=4, Registry=5)

No duplicate discriminants within any single enum.

---

## Upgrade Safety

When upgrading contracts:
1. **Never change the order** of enum variants — this changes discriminants
2. **Never remove variants** — existing data would become unreadable
3. **Only append new variants** at the end of the enum
4. Instance and persistent storage TTLs are extended on every write via `bump_instance` / `bump_persistent`

---

## TTL Configuration (All Contracts)

| Storage Tier | Threshold | Extension |
|--------------|-----------|-----------|
| Instance | 6 days (103,680 ledgers) | 7 days (120,960 ledgers) |
| Persistent | 30 days (518,400 ledgers) | 30 days (518,400 ledgers) |

---

## Key Generation Patterns

### Registry
- `Record(id)` — single u64
- `AllSubscriptions` — static
- `Plan(plan_id)` — single u64

### Subscription
- `Sub(id)` — single u64
- `BySubscriber(addr)` — Address (32 bytes)
- `ByMerchant(addr)` — Address (32 bytes)

### Plan
- `Plan(id)` — single u64
- `ByMerchant(addr)` — Address (32 bytes)
- `AllPlans` — static

---

## Verification Checklist

- [ ] All `DataKey` enums have unique discriminants per contract
- [ ] No variant removed or reordered in existing enums
- [ ] Instance/persistent separation maintained
- [ ] TTL bumping implemented for all keys
- [ ] Cross-contract isolation verified (different contract addresses)
- [ ] Documentation updated with any new keys

---

*Generated for issue #82 — Review storage keys for collisions and document layout*