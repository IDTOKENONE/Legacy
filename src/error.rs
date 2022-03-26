use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.

    #[error("Contract has been locked.")]
    Locked {},

    #[error("Address is not whitelisted.")]
    NotWhitelisted {},

    #[error("Minimum Allocation is 15%")]
    MinimumAllocation {},

    #[error("You must send Luna and only Luna with a deposit message.")]
    NoLunaReceived {},
}
