#![no_std]
//! # Registry Contract
//!
//! A read-optimised index of plans and subscriptions with aggregate stats.

mod errors;
mod types;

pub use errors::Error;
pub use types::{RegistryStats, SubscriptionRecord, Writers};
