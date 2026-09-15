#![no_std]
//! # Plan Contract
//!
//! Merchants publish reusable billing terms that subscribers subscribe to.

mod errors;
mod types;

pub use errors::Error;
pub use types::Plan;
