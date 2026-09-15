#![no_std]
//! # Subscription Contract
//!
//! Recurring pull payments on Soroban. A subscriber grants a bounded,
//! revocable authorization once; the merchant then pulls a fixed amount per
//! billing interval.

mod errors;
mod storage;
mod types;

pub use errors::Error;
pub use types::{SubStatus, Subscription};
