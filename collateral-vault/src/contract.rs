#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{BalanceResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{State, ADMIN, STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:collateral-funds";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let state = State {
        total_deposit: Uint128::zero(),
        clearing_house: msg.clearing_house,
        denom_stable: msg.denom_stable,
    };

    STATE.save(deps.storage, &state)?;
    ADMIN.set(deps.branch(), Some(info.sender.clone()))?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("clearing_house", info.sender.clone())
        .add_attribute("admin", info.sender.clone()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateAdmin { new_admin } => {
            let addr = Some(deps.api.addr_validate(&new_admin)?);
            Ok(ADMIN.execute_update_admin(deps, info, addr)?)
        }
        ExecuteMsg::UpdateClearingHouse { new_clearing_house } => {
            change_clearing_house(deps, info, new_clearing_house)
        }
        ExecuteMsg::Deposit {} => deposit(deps, info),
        ExecuteMsg::Withdraw { to_address, amount } => withdraw(deps, info, to_address, amount),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::GetBalance {} => to_binary(&query_balance(deps)?),
    }
}

pub fn change_clearing_house(
    deps: DepsMut,
    info: MessageInfo,
    clearing_house: Addr,
) -> Result<Response, ContractError> {
    ADMIN.assert_admin(deps.as_ref(), &info.sender.clone())?;
    STATE.update(deps.storage, |mut state| -> Result<State, ContractError> {
        state.clearing_house = clearing_house.clone();
        Ok(state)
    })?;
    Ok(Response::new()
        .add_attribute("method", "update_clearing_house")
        .add_attribute("new_clearing_house", clearing_house.clone()))
}

pub fn deposit(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let mut state: State = STATE.load(deps.storage)?;
    if info.sender != state.clearing_house {
        return Err(ContractError::UnauthorizedClearingHouse {});
    }

    if info.funds.len() != 1 {
        return Err(ContractError::InvalidIncomingAsset {});
    }

    if info.funds[0].denom != state.denom_stable {
        return Err(ContractError::InvalidIncomingAsset {});
    }

    state.total_deposit = state.total_deposit.checked_add(info.funds[0].amount)?;
    STATE.update(deps.storage, |_s| -> Result<State, ContractError> {
        Ok(state)
    })?;
    Ok(Response::new()
        .add_attribute("method", "deposit_collateral")
        .add_attribute("amount", info.funds[0].amount))
}

pub fn withdraw(
    deps: DepsMut,
    info: MessageInfo,
    to: Addr,
    amount: u128,
) -> Result<Response, ContractError> {
    let mut state: State = STATE.load(deps.storage)?;
    let amount = Uint128::from(amount);

    if info.sender != state.clearing_house {
        return Err(ContractError::UnauthorizedClearingHouse {});
    }

    if amount.gt(&state.total_deposit) {
        return Err(ContractError::InsufficientFunds {});
    };

    state.total_deposit = state.total_deposit.checked_sub(amount)?;

    let send_tx_msg = BankMsg::Send {
        to_address: to.into_string(),
        amount: coins(amount.u128(), state.denom_stable.clone()),
    };

    STATE.update(deps.storage, |_s| -> Result<State, ContractError> {
        Ok(state)
    })?;

    Ok(Response::new()
        .add_message(send_tx_msg)
        .add_attribute("method", "withdraw_collateral")
        .add_attribute("amount", amount))
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = STATE.load(deps.storage)?;
    let res = ADMIN.query_admin(deps).unwrap();
    Ok(ConfigResponse {
        clearing_house: state.clearing_house,
        admin: res.admin.unwrap(),
        denom: state.denom_stable,
    })
}

fn query_balance(deps: Deps) -> StdResult<BalanceResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(BalanceResponse {
        balance: state.total_deposit,
    })
}
