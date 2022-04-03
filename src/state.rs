use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use crate::MINIMUM_COMMISSION;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub trader: Addr,
    pub trader_withdrawal_address: Addr,
    pub funder: Addr,
    pub whitelist: Vec<Addr>,
    pub native_tokens: Vec<String>,
    pub cw20_tokens: Vec<Addr>,
    pub trader_lock: bool,
    pub funder_lock: bool,
    pub base_investment: Uint128,
    pub commission: u8,
}


impl State {
    pub fn new(trader: Addr, funder: Addr) -> State {
        let withdrawal_address = trader.clone();
        State {
            trader,
            trader_withdrawal_address: withdrawal_address,
            funder,
            whitelist: vec![],
            native_tokens: vec![],
            cw20_tokens: vec![],
            trader_lock: false,
            funder_lock: false,
            base_investment: Uint128::new(0),
            commission: MINIMUM_COMMISSION,
        }
    }
}

pub const STATE: Item<State> = Item::new("state");
