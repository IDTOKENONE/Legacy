use cosmwasm_std::{QuerierWrapper, Addr, StdResult, Uint128, QueryRequest, WasmQuery, to_binary, CosmosMsg, WasmMsg, Response, Binary};
use cw20::{Cw20QueryMsg, BalanceResponse, Cw20ExecuteMsg};

pub fn query_token_balance(
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

pub fn add_cw20_msg<T1, T2>(
    res: Response,
    contract_addr: T1,
    recipient: T2,
    amount: Uint128,
    msg: Option<Binary>,
) -> Response 
where
    T1: ToString,
    T2: ToString,
{
    match msg {
        Some(msg) => {
            res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Send {
                    amount,
                    contract: recipient.to_string(),
                    msg,
                }).unwrap(),
            }))
        },
        None => {
            res.add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount,
                }).unwrap(),
            }))
        }
    }
}
