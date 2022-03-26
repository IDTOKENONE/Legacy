use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub user: Addr,
    pub whitelist: Vec<Addr>,
    pub cw20_tokens: Vec<Addr>,
    pub lock: bool,
    pub base_investment: Uint128,
    pub commission: u8,
}


impl State {
    pub fn initial(owner: Addr, user: Addr) -> State {
        State {
            owner,
            user,
            whitelist: vec![],
            cw20_tokens: vec![],
            lock: false,
            base_investment: Uint128::new(0),
            commission: 15,
        }
    }
}

pub const STATE: Item<State> = Item::new("state");
