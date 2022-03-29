use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::Item;
use crate::MINIMUM_COMMISSION;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub owner: Addr,
    pub owner_withdrawal_address: Addr,
    pub user: Addr,
    pub whitelist: Vec<Addr>,
    pub native_tokens: Vec<String>,
    pub cw20_tokens: Vec<Addr>,
    pub lock: bool,
    pub base_investment: Uint128,
    pub commission: u8,
}


impl State {
    pub fn initial(owner: Addr, user: Addr) -> State {
        let withdrawal_address = owner.clone();
        State {
            owner,
            owner_withdrawal_address: withdrawal_address,
            user,
            whitelist: vec![],
            native_tokens: vec![],
            cw20_tokens: vec![],
            lock: false,
            base_investment: Uint128::new(0),
            commission: MINIMUM_COMMISSION,
        }
    }
}

pub const STATE: Item<State> = Item::new("state");
