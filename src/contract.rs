#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Uint128, Decimal, BankMsg, CosmosMsg, Coin, WasmMsg};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse};
use crate::state::{State, STATE, Asset};
use crate::MINIMUM_COMMISSION;
use crate::util::{query_token_balance, add_cw20_msg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:counter";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    // Save who the owner & user are
    let mut state = State::new(
        info.clone().sender,
        msg.clone().funder
    );

    // Assign state items that were specified
    if let Some(whitelist) = msg.whitelist {
        state.whitelist = whitelist;
    }
    if let Some(assets) = msg.assets {
        state.assets = assets;
    }
    if let Some(commission) = msg.commission {
        if commission >= MINIMUM_COMMISSION {
            state.commission = commission;
        }
    }
    if let Some(addr) = msg.trader_withdrawal_address {
        state.trader_withdrawal_address = addr;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("user", msg.funder.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::ToggleLock {} => toggle_lock(deps, info),
        ExecuteMsg::Deposit {} => deposit(deps, info),
        ExecuteMsg::Withdraw {
            amount
            } => withdraw(
                deps,   
                info,   
                env,    
                amount
            ),
        ExecuteMsg::UpdateState {
            whitelist,
            assets,
            commission,
            user,
            } => update_state(
                deps,
                info,
                env,
                whitelist,
                assets,
                commission,
                user,
            ),
        ExecuteMsg::SendNative {
            address,
            funds,
            msg,
            } => send_native(
                deps,
                info,
                address,
                funds,
                msg,
            ),
        ExecuteMsg::SendCw20 {
            address,
            token_addr,
            amount,
            msg
            } => send_cw20(
                deps,
                info,
                address,
                token_addr,
                amount,
                msg
            ),
        ExecuteMsg::UpdateWithdrawal {
            address,
            } => update_owner_withdrawal(
                deps,
                info,
                address,
            ),
    }
}

fn update_owner_withdrawal(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage).unwrap();

    if info.sender != state.trader {
        return Err(ContractError::Unauthorized {})
    }

    state.trader_withdrawal_address = address;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("method", "update_owner_withdrawal"))
}


fn send_native(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
    funds: Option<Vec<Coin>>,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage).unwrap();

    // Check if sender is owner
    if info.sender != state.trader {
        return Err(ContractError::Unauthorized {})
    }

    // Check if address is whitelisted
    if !state.whitelist.contains(&address) {
        return Err(ContractError::NotWhitelisted {})
    }

    let mut msg_funds = vec![];

    if let Some(val) = funds {
        msg_funds = val;
    }

    match msg {
        Some(msg) => {
            let final_msg = CosmosMsg::Wasm(WasmMsg::Execute{
                contract_addr: address.into_string(),
                msg,
                funds: msg_funds,
            });
            return Ok(Response::new()
                .add_attribute("method", "send_native")
                .add_attribute("sent_to", "wasm_contract")
                .add_message(final_msg))

        },
        None => {
            let final_msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: address.into_string(),
                amount: msg_funds,
            });
            return Ok(Response::new()
                .add_attribute("method", "send_native")
                .add_attribute("sent_to", "wallet")
                .add_message(final_msg))
        }
    }
}

fn send_cw20(
    deps: DepsMut,
    info: MessageInfo,
    address: Addr,
    token_addr: Addr,
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage).unwrap();

    if info.sender != state.trader {
        return Err(ContractError::Unauthorized {})
    }

    if !state.whitelist.contains(&address) {
        return Err(ContractError::NotWhitelisted {})
    }

    match msg {
        Some(msg) => {
            let final_msg = CosmosMsg::Wasm(WasmMsg::Execute{
                contract_addr: token_addr.into_string(),
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    contract: address.into_string(),
                    amount,
                    msg,})?,
                funds: vec![],
            }
            );
            return Ok(Response::new()
                .add_attribute("method", "send_cw20")
                .add_attribute("sent_to", "wasm_contract")
                .add_message(final_msg))
        },
        None => {
            let final_msg = CosmosMsg::Wasm(WasmMsg::Execute{
                contract_addr: token_addr.into_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: address.into_string(),
                    amount
                })?,
                funds: vec![],
            });
            return Ok(Response::new()
                .add_attribute("method", "send_cw20")
                .add_attribute("sent_to", "wallet")
                .add_message(final_msg))
        }
    }
}

fn deposit(
    deps: DepsMut,
    info: MessageInfo
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage).unwrap();
    // Check if address is user
    if info.sender != state.funder {
        // If sender is not user
        return Err(ContractError::Unauthorized {})
    }

    let funds = info.funds;

    // Check that only one kind of coin was sent
    if funds.len() != 1 {
        // If not
        return Err(ContractError::NoLunaReceived {})
    }

    // Check that the currency received is Luna
    if funds[0].denom != String::from("uluna") {
        return Err(ContractError::NoLunaReceived {})
    }
    
    let luna_sent = funds[0].amount;

    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        // Add Luna amount to base_investment
        state.base_investment = state.base_investment + luna_sent;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "deposit"))
}

fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    amount: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage).unwrap();

    if 
        info.sender != state.funder && 
        info.sender != state.trader && 
        info.sender != state.trader_withdrawal_address 
    {
        return Err(ContractError::Unauthorized {})
    }

    let mut res = Response::new()
        .add_attribute("method", "withdraw");

    let mut total_balance = Uint128::zero();
    let mut assets: Vec<(Asset, Uint128)> = vec![];

    for asset in state.assets.clone() {
        match asset.clone() {
            Asset::Native(denom) => {
                let balance = deps.querier.query_balance(env.contract.address.clone(), denom);
                if let Ok(coin) = balance {
                    total_balance += coin.amount;
                    assets.push((asset, coin.amount));
                }
            },
            Asset::Token(address) => {
                let balance = query_token_balance(&deps.querier, address.clone(), env.contract.address.clone());
                if let Ok(bal) = balance {
                    total_balance += bal;
                    assets.push((asset, bal));
                }
            },
        }
    }

    let profit = total_balance - state.base_investment;
    let trader_percent = Decimal::percent(state.commission.into());
    let mut trader_funds = profit * trader_percent;
    let mut funder_withdrawal = total_balance - trader_funds;

    let mut trader_coins = vec![];
    let mut funder_coins = vec![];

    if let Some(amt) = amount {
        funder_withdrawal = amt;
    }

    state.base_investment = total_balance - trader_funds - funder_withdrawal;

    'outer: for asset in assets {
        match asset {
            (Asset::Native(denom), amt) => {
                if trader_funds >= amt {
                    // All of this asset goes to trader
                    trader_coins.push(Coin {
                        denom,
                        amount: amt,
                    });
                    trader_funds -= amt;
                } else if !trader_funds.is_zero() {
                    // Only *some* of this asset goes to trader
                    trader_coins.push(Coin {
                        denom: denom.clone(),
                        amount: trader_funds,
                    });
                    
                    // Check for withdrawal to funder
                    if funder_withdrawal >= amt - trader_funds {
                        // All remaining goes to funder
                        funder_coins.push(Coin {
                            denom,
                            amount: amt - trader_funds,
                        });
                        funder_withdrawal -= amt - trader_funds;
                    } else if !funder_withdrawal.is_zero() {
                        // Some of this asset goes to funder
                        funder_coins.push(Coin {
                            denom,
                            amount: funder_withdrawal,
                        });
                        // funder_withdrawal = Uint128::zero();
                        break 'outer
                    }

                    trader_funds = Uint128::zero();
                } else if funder_withdrawal >= amt {
                    // All of this asset goes to funder
                    funder_coins.push(Coin {
                        denom,
                        amount: amt,
                    });
                    funder_withdrawal -= amt;
                } else if !funder_withdrawal.is_zero() {
                    // Some of this asset goes to funder
                    funder_coins.push(Coin {
                        denom,
                        amount: funder_withdrawal,
                    });
                    // funder_withdrawal = Uint128::zero();
                    break 'outer;
                } else {
                    break 'outer;
                }
            },
            (Asset::Token(addr), amt) => {
                if trader_funds >= amt {
                    // All of this asset goes to trader
                    res = add_cw20_msg(
                        res,
                        addr,
                        state.trader_withdrawal_address.clone(),
                        amt,
                        None,
                    );
                    trader_funds -= amt;
                } else if !trader_funds.is_zero() {
                    // Only *some* of this asset goes to trader
                    res = add_cw20_msg(
                        res,
                        addr.clone(),
                        state.trader_withdrawal_address.clone(),
                        trader_funds,
                        None
                    );

                    // Check for withdrawal to funder
                    if funder_withdrawal >= amt - trader_funds {
                        // All remaining goes to funder
                        res = add_cw20_msg(
                            res,
                            addr,
                            state.funder.clone(),
                            amt - trader_funds,
                            None,
                        );
                        funder_withdrawal -= amt - trader_funds;
                    } else if !funder_withdrawal.is_zero() {
                        // Some of this asset goes to funder
                        res = add_cw20_msg(
                            res,
                            addr,
                            state.funder.clone(),
                            funder_withdrawal,
                            None,
                        );
                        // funder_withdrawal = Uint128::zero();
                        break 'outer;
                    }
                } else if funder_withdrawal >= amt {
                    // All of this asset goes to funder
                    res = add_cw20_msg(
                        res,
                        addr,
                        state.funder.clone(),
                        amt,
                        None,
                    );
                    funder_withdrawal -= amt;
                } else if !funder_withdrawal.is_zero() {
                    // Some of this asset goes to funder
                    res = add_cw20_msg(
                        res,
                        addr,
                        state.funder.clone(),
                        funder_withdrawal,
                        None,
                    );
                    // funder_withdrawal = Uint128::zero();
                    break 'outer;
                } else {
                    break 'outer;
                }
            },
        }
    }

    // Send coins
    if trader_coins.len() > 0 {
        res = res.add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: state.trader_withdrawal_address.to_string(),
            amount: trader_coins,
        }));
    };
    if funder_coins.len() > 0 {
        res = res.add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: state.funder.to_string(),
            amount: funder_coins,
        }));
    };

    STATE.save(deps.storage, &state)?;

    Ok(res)
}

fn toggle_lock(
    deps: DepsMut,
    info: MessageInfo
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage).unwrap();

    if info.sender == state.funder {
        state.funder_lock = !state.funder_lock;
        STATE.save(deps.storage, &state)?;
        return Ok(Response::new()
            .add_attribute("method", "toggle_lock")
            .add_attribute("funder_lock", state.funder_lock.to_string())
            .add_attribute("trader_lock", state.trader_lock.to_string())
            .add_attribute("locked", (state.trader_lock || state.funder_lock).to_string()))
    }

    if info.sender == state.trader {
        state.trader_lock = !state.trader_lock;
        STATE.save(deps.storage, &state)?;
        return Ok(Response::new()
            .add_attribute("method", "toggle_lock")
            .add_attribute("funder_lock", state.funder_lock.to_string())
            .add_attribute("trader_lock", state.trader_lock.to_string())
            .add_attribute("locked", (state.trader_lock || state.funder_lock).to_string()))
    }

    Err(ContractError::Unauthorized {})

}

fn update_state(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    whitelist: Option<Vec<Addr>>,
    assets: Option<Vec<Asset>>,
    commission: Option<u8>,
    user: Option<Addr>,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage).unwrap();

    // If sender is trader and funder has locked
    if info.sender == state.trader {
        if state.funder_lock {
            return Err(ContractError::Locked {})
        }
    // If sender is funder and trader has locked
    } else if info.sender == state.funder {
        if state.trader_lock {
            return Err(ContractError::Locked {})
        }
    // If sender is neither of funder or trader
    } else {
        return Err(ContractError::Unauthorized {})
    }

    // Update all included values in state
    if let Some(val) = whitelist {
        state.whitelist = val;
    };
    if let Some(val) = assets {
        state.assets = val;
    }
    if let Some(val) = commission {
        if val >= MINIMUM_COMMISSION {
            state.commission = val;
        }
    };
    if let Some(val) = user {
        // Check that there are currently no funds in the contract
        let mut total_balance = Uint128::zero();
        for asset in state.assets.clone() {
            match asset {
                Asset::Native(denom) => {
                    let balance = deps.querier.query_balance(env.contract.address.clone(), denom);
                    if let Ok(coin) = balance {
                        total_balance += coin.amount;
                    }
                },
                Asset::Token(address) => {
                    let balance = query_token_balance(&deps.querier, address.clone(), env.contract.address.clone());
                    if let Ok(bal) = balance {
                        total_balance += bal;
                    }
                },
            }
        }        if total_balance.is_zero() {
            state.funder = val;
        }
    }

    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("method", "update_state"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(
    deps: Deps,
    _env: Env,
    msg: QueryMsg
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
    }
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse { state })
}

#[cfg(test)]
mod tests {
}
