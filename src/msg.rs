use cosmwasm_std::{Addr, Uint128, Binary, Coin};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{State, Asset};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigItem {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub funder: Addr,
    pub trader_withdrawal_address: Option<Addr>,
    pub whitelist: Option<Vec<Addr>>,
    pub assets: Option<Vec<Asset>>,
    pub commission: Option<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ToggleLock {},
    UpdateState { whitelist: Option<Vec<Addr>>, assets: Option<Vec<Asset>>, commission: Option<u8>, user: Option<Addr> },
    SendNative{ address: Addr, funds: Option<Vec<Coin>>, msg: Option<Binary> },
    SendCw20 { address: Addr, token_addr: Addr, amount: Uint128, msg: Option<Binary> },
    Deposit {},
    Withdraw { amount: Option<Uint128> },
    UpdateWithdrawal { address: Addr },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetState returns the state data such as whitelisted addresses,
    // profit_allocation, user/owner, etc.
    GetState {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    pub state: State,
}
