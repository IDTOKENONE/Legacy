#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, Uint128, QuerierWrapper, QueryRequest, WasmQuery, Decimal, BankMsg, CosmosMsg, Coin, WasmMsg};
use cw2::set_contract_version;
use cw20::{BalanceResponse, Cw20QueryMsg, Cw20ExecuteMsg};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, StateResponse};
use crate::state::{State, STATE};

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
    let state = State::initial(info.clone().sender, msg.clone().user);

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
        ExecuteMsg::Trade { address, msg, luna_amt } => try_trade(deps, info, address, msg, luna_amt),
        ExecuteMsg::Deposit {} => try_deposit(deps, info),
        ExecuteMsg::Withdraw {} => try_withdraw(deps, info, env),
        ExecuteMsg::ToggleLock {  } => try_toggle_lock(deps, info),
        ExecuteMsg::Whitelist { addresses } => try_whitelist(deps, info, addresses),
        ExecuteMsg::RegisterTokens { addresses } => try_register_tokens(deps, info, addresses),
        ExecuteMsg::AdjustDistribution { amt } => try_adjust_distribution(deps, info, amt),
    }
}

fn try_trade(deps: DepsMut, info: MessageInfo, address: Addr, msg: Binary, luna_amt: Uint128) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage).unwrap();

    // Check if sender is owner or user
    if info.sender != state.owner && info.sender != state.user {
        // If neither
        return Err(ContractError::Unauthorized {})
    }

    // Check if address is whitelisted
    if !state.whitelist.contains(&address) {
        return Err(ContractError::NotWhitelisted {})
    }

    let mut funds = vec![];

    if !luna_amt.is_zero() {
        funds.push(Coin {
            denom: String::from("uluna"),
            amount: luna_amt,
        })
    }

    let msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: address.into_string(),
        msg: msg,
        funds: funds,
    });

    Ok(Response::new()
        .add_attribute("method", "try_trade")
        .add_message(msg)
    )
}

fn try_deposit(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
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
    Ok(Response::new().add_attribute("method", "try_deposit"))
}

fn try_withdraw(deps: DepsMut, info: MessageInfo, env: Env) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage).unwrap();

    // Check if sender is user or owner
    if info.sender != state.user && info.sender != state.owner {
        // Is neither
        return Err(ContractError::Unauthorized {})
    }

    // Will store messages to be sent
    let mut messages: Vec<CosmosMsg> = vec![];

    // Start with zero
    let mut total_balance = Uint128::zero();
    let mut total_luna = Uint128::zero();

    // Get Luna Balance
    let luna_balance = deps.querier.query_balance(env.contract.address.clone(), String::from("uluna"));
    // Add Luna Balance to total_balance
    if let Ok(coin) = luna_balance {
        total_balance += coin.amount;
        total_luna += coin.amount;
    }

    let mut cw20_balances = vec![];

    // Get balance of registered tokens and add them to total_balance
    for token_addr in state.cw20_tokens {
        // Get token balance
        let token_balance = query_token_balance(
            &deps.querier,
            token_addr.clone(),
            env.contract.address.clone());
        
        // If balance is returned, add to total_balance
        if let Ok(balance) = token_balance {
            total_balance += balance;
            cw20_balances.push((token_addr.clone(), balance));
        }
    }

    // Determine owner allocation. Formula should be
    // (Totalluna - STATE.base_investment) * STATE.profit_alloc
    let profit = total_balance - state.base_investment;
    let owner_percent = Decimal::percent(state.commission.into());
    let mut owner_withdrawal = profit * owner_percent;

    // If the amount to be withdrawn to owner is more than the total luna
    if owner_withdrawal >= total_luna {
        // Withdraw all held Luna to owner
        messages.push(CosmosMsg::Bank(BankMsg::Send {
            to_address: state.owner.clone().into_string(),
            amount: vec![Coin { denom: String::from("uluna"), amount: total_luna }],
        }));

        // Reduce amount to be withdrawn to owner
        owner_withdrawal -= total_luna;
    // If the amount to be withdrawn is less than the total luna
    } else {
        // If there are still funds left to withdraw to owner
        if !owner_withdrawal.is_zero() {
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: state.owner.clone().into_string(),
                amount: vec![Coin { denom: String::from("uluna"), amount: owner_withdrawal }],
            }));
        }
        // If the person calling withdraw is the sender
        if info.sender == state.user {
            // Withdraw remaining luna to sender
            messages.push(CosmosMsg::Bank(BankMsg::Send {
                to_address: state.user.clone().into_string(),
                amount: vec![Coin { denom: String::from("uluna"), amount: total_luna - owner_withdrawal }],
            }));
        };

        // Set remaining owner withdrawal to 0
        owner_withdrawal = Uint128::zero();
    }

    for token_balance in cw20_balances {
        // If the amount to be withdrawn is more than the total token balance
        if owner_withdrawal >= token_balance.1 {
            // Withdraw all of token to owner
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token_balance.0.into_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: state.owner.clone().into_string(),
                    amount: token_balance.1,
                })?,
                funds: vec![],
            }));

            owner_withdrawal -= token_balance.1;
        // If the amount to be withdrawn is less than the total amount of the token
        } else {
            // If there are still funds left to withdraw to owner
            if !owner_withdrawal.is_zero() {
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: token_balance.0.clone().into_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: state.owner.clone().into_string(),
                        amount: owner_withdrawal,
                    })?,
                    funds: vec![],
                }));
            }
            // If the person calling withdraw is the sender
            if info.sender == state.user {
                // Withdraw remaining token to sender
                messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: token_balance.0.clone().into_string(),
                    msg: to_binary(&Cw20ExecuteMsg::Transfer {
                        recipient: state.user.clone().into_string(),
                        amount: token_balance.1 - owner_withdrawal,
                    })?,
                    funds: vec![],
                }));
            };

            // Set remaining owner withdrawal to 0
            owner_withdrawal = Uint128::zero();
            // Withdraw remaining token to sender
        }
    }

    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        // Set base_investment to remaining_luna
        state.base_investment = Uint128::zero();
        Ok(state)
    })?;

    Ok(Response::new()
        .add_attribute("method", "try_withdraw")
        .add_messages(messages)
    )
}

fn try_toggle_lock(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
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
    
    Ok(Response::new().add_attribute("method", "try_toggle_lock"))
}

fn try_whitelist(deps: DepsMut, info: MessageInfo, mut addresses: Vec<Addr>) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage).unwrap();
    
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

    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.whitelist.append(&mut addresses);
        Ok(state)
    })?;
        
    Ok(Response::new().add_attribute("method", "try_whitelist"))
}

fn try_register_tokens(deps: DepsMut, info: MessageInfo, mut addresses: Vec<Addr>) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage).unwrap();

    // Check if sender is owner
    if info.sender == state.owner {
        // Check if STATE.lock
        if state.lock {
            return Err(ContractError::Locked {})
        }
        // Check if sender is user
    } else if info.sender != state.user {
        // If not
        return Err(ContractError::Unauthorized {})
    }

    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.cw20_tokens.append(&mut addresses);
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "try_register_tokens"))
}

fn try_adjust_distribution(deps: DepsMut, info: MessageInfo, amt: u8 ) -> Result<Response, ContractError> {
    let state = STATE.load(deps.storage).unwrap();

    // Check if sender is owner or user
    if info.sender != state.owner && info.sender != state.user {
        // If not
        return Err(ContractError::Unauthorized {})
    }

    // Check if amount is at least 15
    if amt < 15 {
        // If not
        return Err(ContractError::MinimumAllocation {})
    }

    // Update profit allocation to be sent amount
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.commission = amt;
        Ok(state)
    })?;
    
    Ok(Response::new().add_attribute("method", "try_adjust_distribution"))
}

fn query_token_balance(querier: &QuerierWrapper, contract_addr: Addr, account_addr: Addr) -> StdResult<Uint128> {
    let res: BalanceResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: contract_addr.to_string(),
        msg: to_binary(&Cw20QueryMsg::Balance {
            address: account_addr.to_string(),
        })?,
    }))?;

    Ok(res.balance)
}


// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn execute(
//     deps: DepsMut,
//     _env: Env,
//     info: MessageInfo,
//     msg: ExecuteMsg,
// ) -> Result<Response, ContractError> {
//     match msg {
//         ExecuteMsg::Increment {} => try_increment(deps),
//         ExecuteMsg::Reset { count } => try_reset(deps, info, count),
//     }
// }

// pub fn try_increment(deps: DepsMut) -> Result<Response, ContractError> {
//     STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
//         state.count += 1;
//         Ok(state)
//     })?;

//     Ok(Response::new().add_attribute("method", "try_increment"))
// }
// pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
//     STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
//         if info.sender != state.owner {
//             return Err(ContractError::Unauthorized {});
//         }
//         state.count = count;
//         Ok(state)
//     })?;
//     Ok(Response::new().add_attribute("method", "reset"))
// }

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetState {} => to_binary(&query_state(deps)?),
    }
}

fn query_state(deps: Deps) -> StdResult<StateResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(StateResponse { state })
}

// #[cfg_attr(not(feature = "library"), entry_point)]
// pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
//     match msg {
//         QueryMsg::GetCount {} => to_binary(&query_count(deps)?),
//     }
// }

// fn query_count(deps: Deps) -> StdResult<CountResponse> {
//     let state = STATE.load(deps.storage)?;
//     Ok(CountResponse { count: state.count })
// }

#[cfg(test)]
mod tests {
    // use super::*;
    // use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    // use cosmwasm_std::{coins, from_binary};

    // #[test]
    // fn proper_initialization() {
    //     let mut deps = mock_dependencies(&[]);

    //     let msg = InstantiateMsg { count: 17 };
    //     let info = mock_info("creator", &coins(1000, "earth"));

    //     // we can just call .unwrap() to assert this was a success
    //     let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    //     assert_eq!(0, res.messages.len());

    //     // it worked, let's query the state
    //     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(17, value.count);
    // }

    // #[test]
    // fn increment() {
    //     let mut deps = mock_dependencies(&coins(2, "token"));

    //     let msg = InstantiateMsg { count: 17 };
    //     let info = mock_info("creator", &coins(2, "token"));
    //     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    //     // beneficiary can release it
    //     let info = mock_info("anyone", &coins(2, "token"));
    //     let msg = ExecuteMsg::Increment {};
    //     let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    //     // should increase counter by 1
    //     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(18, value.count);
    // }

    // #[test]
    // fn reset() {
    //     let mut deps = mock_dependencies(&coins(2, "token"));

    //     let msg = InstantiateMsg { count: 17 };
    //     let info = mock_info("creator", &coins(2, "token"));
    //     let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

    //     // beneficiary can release it
    //     let unauth_info = mock_info("anyone", &coins(2, "token"));
    //     let msg = ExecuteMsg::Reset { count: 5 };
    //     let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
    //     match res {
    //         Err(ContractError::Unauthorized {}) => {}
    //         _ => panic!("Must return unauthorized error"),
    //     }

    //     // only the original creator can reset the counter
    //     let auth_info = mock_info("creator", &coins(2, "token"));
    //     let msg = ExecuteMsg::Reset { count: 5 };
    //     let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

    //     // should now be 5
    //     let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
    //     let value: CountResponse = from_binary(&res).unwrap();
    //     assert_eq!(5, value.count);
    // }
}
