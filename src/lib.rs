pub mod contract;
mod error;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;

pub const MINIMUM_COMMISSION: u8 = 20;
