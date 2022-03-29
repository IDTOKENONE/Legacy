#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Uint128, QuerierWrapper, QueryRequest, WasmQuery, Decimal, BankMsg, CosmosMsg, Coin, WasmMsg};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20QueryMsg, Cw20ExecuteMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse};
use crate::state::{State, STATE};
use crate::MINIMUM_COMMISSION;

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
    let mut state = State::initial(
        info.clone().sender,
        msg.clone().user
    );

    // Assign state items that were specified
    if let Some(whitelist) = msg.whitelist {
        state.whitelist = whitelist;
    }
    if let Some(native_tokens) = msg.native_tokens {
        state.native_tokens = native_tokens;
    }
    if let Some(cw20_tokens) = msg.cw20_tokens {
        state.cw20_tokens = cw20_tokens;
    }
    if let Some(commission) = msg.commission {
        if commission >= MINIMUM_COMMISSION {
            state.commission = commission;
        }
    }
    if let Some(addr) = msg.owner_withdrawal_address {
        state.owner_withdrawal_address = addr;
    }

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("user", msg.user.to_string()))
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
            native_tokens,
            cw20_tokens,
            commission,
            user,
            } => update_state(
                deps,
                info,
                env,
                whitelist,
                native_tokens,
                cw20_tokens,
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
    }
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
    if info.sender != state.owner {
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

    if info.sender != state.owner {
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
    if info.sender != state.user {
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

    let mut res = Response::new()
        .add_attribute("method", "withdraw");

    let mut total_balance = Uint128::zero();
    let mut coins = vec![];
    let mut tokens = vec![];


    for denom in state.native_tokens.clone() {
        let balance = deps.querier.query_balance(env.contract.address.clone(), denom);
        if let Ok(coin) = balance {
            total_balance += coin.amount;
            coins.push(coin);
        }
    }

    for token_addr in state.cw20_tokens.clone() {
        let balance = query_token_balance(&deps.querier, token_addr.clone(), env.contract.address.clone());
        if let Ok(bal) = balance {
            total_balance += bal;
            tokens.push((token_addr, bal));
        }
    }

    let profit = total_balance - state.base_investment;
    let owner_percent = Decimal::percent(state.commission.into());
    let mut owner_withdrawal_amount = profit * owner_percent;
    let mut user_withdrawal_amount = amount.clone();

    match amount {
        Some(amt) => {
            if owner_withdrawal_amount + amt > total_balance {
                state.base_investment = Uint128::zero();
            } else {
                state.base_investment = state.base_investment - owner_withdrawal_amount - amt;
            }
        },
        None => {
            state.base_investment = Uint128::zero();
        }
    }

    let mut owner_withdrawal_coins = vec![];
    let mut user_withdrawal_coins = vec![];

    // Withdraw coins
    'outer_coins: for coin in coins {
        // All of owner allocation has been withdrawn
        if owner_withdrawal_amount.is_zero() {
            if info.sender == state.user {
                match user_withdrawal_amount {
                    Some(withdraw_amt) => {
                        if coin.amount < withdraw_amt {
                            user_withdrawal_coins.push(Coin {
                                denom: coin.denom,
                                amount: coin.amount,
                            });
                            user_withdrawal_amount = Some(withdraw_amt - coin.amount);
                        } else {
                            user_withdrawal_coins.push(Coin {
                                denom: coin.denom,
                                amount: withdraw_amt,
                            });
                            // user_withdrawal_amount = Some(Uint128::zero());
                            break 'outer_coins;
                        }
                    },
                    None => {
                        user_withdrawal_coins.push(Coin {
                            denom: coin.denom,
                            amount: coin.amount,
                        });
                    }
                }
            }
        // All of coin will be sent to owner
        } else if coin.amount < owner_withdrawal_amount {
            owner_withdrawal_coins.push(Coin { denom: coin.denom, amount: coin.amount });
            owner_withdrawal_amount -= coin.amount;
        // Some of coin will be sent to owner, and some to user
        } else {
            owner_withdrawal_coins.push(Coin { denom: coin.denom.clone(), amount: owner_withdrawal_amount.clone() });
            if info.sender == state.user {
                let coin_amount = coin.amount - owner_withdrawal_amount;
                match user_withdrawal_amount {
                    Some(withdraw_amt) => {
                        if coin_amount < withdraw_amt {
                            user_withdrawal_coins.push(Coin {
                                denom: coin.denom,
                                amount: coin_amount,
                            });
                            user_withdrawal_amount = Some(withdraw_amt - coin_amount);
                        } else {
                            user_withdrawal_coins.push(Coin {
                                denom: coin.denom,
                                amount: withdraw_amt,
                            });
                            // user_withdrawal_amount = Some(Uint128::zero());
                            break 'outer_coins;
                        }
                    },
                    None => {
                        user_withdrawal_coins.push(Coin {
                            denom: coin.denom,
                            amount: coin.amount,
                        });
                    }
                }
            }
            owner_withdrawal_amount = Uint128::zero();
        }
    }
    
    if owner_withdrawal_coins.len() > 0 {
        res = res.add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: state.owner_withdrawal_address.clone().into_string(),
            amount: owner_withdrawal_coins,
        }));
    }
    if user_withdrawal_coins.len() > 0 && info.sender == state.user {
        res = res.add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: state.user.clone().into_string(),
            amount: user_withdrawal_coins,
        }))
    }

    // Withdraw tokens
    'outer_token: for (token_addr, token_amt) in tokens {
        // All of token will be sent to user
        if owner_withdrawal_amount.is_zero() {
            if info.sender == state.user {
                match user_withdrawal_amount {
                    Some(withdraw_amt) => {
                        if token_amt < withdraw_amt {
                            res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: token_addr.into_string(),
                                funds: vec![],
                                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                    recipient: state.user.clone().into_string(),
                                    amount: token_amt,
                                }).unwrap(),
                            }));
                            user_withdrawal_amount = Some(withdraw_amt - token_amt);
                        } else {
                            res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: token_addr.into_string(),
                                funds: vec![],
                                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                    recipient: state.user.clone().into_string(),
                                    amount: withdraw_amt,
                                }).unwrap(),
                            }));
                            // user_withdrawal_amount = Some(Uint128::zero());
                            break 'outer_token;
                        }
                    },
                    None => {
                        res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: token_addr.into_string(),
                            funds: vec![],
                            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                recipient: state.user.clone().into_string(),
                                amount: token_amt,
                            }).unwrap(),
                        }));
                    }
                }
            }
        // All of token will be sent to owner
        } else if token_amt < owner_withdrawal_amount {
            res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute{
                contract_addr: token_addr.into_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: state.owner_withdrawal_address.clone().into_string(),
                    amount: token_amt,
                }).unwrap(),
            }));
            owner_withdrawal_amount -= token_amt;
        // Some of token will be sent to owner, and some to user
        } else {
            res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_addr.clone().into_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: state.owner_withdrawal_address.clone().into_string(),
                    amount: owner_withdrawal_amount.clone(),
                }).unwrap(),
            }));
            let token_amt = token_amt - owner_withdrawal_amount;
            if info.sender == state.user {
                match user_withdrawal_amount {
                    Some(withdraw_amt) => {
                        if token_amt < withdraw_amt {
                            res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: token_addr.into_string(),
                                funds: vec![],
                                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                    recipient: state.user.clone().into_string(),
                                    amount: token_amt,
                                }).unwrap(),
                            }));
                            user_withdrawal_amount = Some(withdraw_amt - token_amt);
                        } else {
                            res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                                contract_addr: token_addr.into_string(),
                                funds: vec![],
                                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                    recipient: state.user.clone().into_string(),
                                    amount: withdraw_amt,
                                }).unwrap(),
                            }));
                            // user_withdrawal_amount = Some(Uint128::zero());
                            break 'outer_token;
                        }
                    },
                    None => {
                        res = res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: token_addr.into_string(),
                            funds: vec![],
                            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                                recipient: state.user.clone().into_string(),
                                amount: token_amt - owner_withdrawal_amount,
                            }).unwrap(),
                        }));        
                    }
                }
            };
            owner_withdrawal_amount = Uint128::zero();
        }
    }

    STATE.save(deps.storage, &state)?;

    Ok(res)
}

fn toggle_lock(
    deps: DepsMut,
    info: MessageInfo
) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage).unwrap();

    // Check if sender is user
    if info.sender != state.user {
        // If sender is not user
        return Err(ContractError::Unauthorized {})
    }

    // Flip lock between true/false
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.lock = !state.lock;
        Ok(state)
    })?;
    
    Ok(Response::new().add_attribute("method", "toggle_lock"))
}

fn update_state(
    deps: DepsMut,
    info: MessageInfo,
    env: Env,
    whitelist: Option<Vec<Addr>>,
    native_tokens: Option<Vec<String>>,
    cw20_tokens: Option<Vec<Addr>>,
    commission: Option<u8>,
    user: Option<Addr>,
) -> Result<Response, ContractError> {
    let mut state = STATE.load(deps.storage).unwrap();

    // Check if sender is owner
    if info.sender == state.owner {
        // Check if STATE.locked
        if state.lock {
            return Err(ContractError::Locked {})
        }
    // Check if sender is user
    } else if info.sender != state.user {
        // If not
        return Err(ContractError::Unauthorized {})
    }

    // Update all included values in state
    if let Some(val) = whitelist {
        state.whitelist = val;
    };
    if let Some(val) = native_tokens {
        state.native_tokens = val;
    };
    if let Some(val) = cw20_tokens {
        state.cw20_tokens = val;
    };
    if let Some(val) = commission {
        if val >= MINIMUM_COMMISSION {
            state.commission = val;
        }
    };
    if let Some(val) = user {
        // Check that there are currently no funds in the contract
        let mut total_balance = Uint128::zero();
        for token_address in &state.cw20_tokens {
            let balance = query_token_balance(
                &deps.querier,
                token_address.clone(),
                env.contract.address.clone()
            );
            if let Ok(bal) = balance {
                total_balance += bal;
            }
        }
        for denom in &state.native_tokens {
            let balance = deps.querier.query_balance(
                env.contract.address.clone(),
                denom,
            );
            if let Ok(coin) = balance {
                total_balance += coin.amount;
            }
        }
        if total_balance.is_zero() {
            state.user = val;
        }
    }

    STATE.save(deps.storage, &state)?;

    Ok(Response::new().add_attribute("method", "update_state"))
}

fn query_token_balance(
    querier: &QuerierWrapper,
    contract_addr: Addr,
    account_addr: Addr
) -> StdResult<Uint128> {
    let res: BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_addr.to_string(),
        })?,
    }))?;

    Ok(res.balance)
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
