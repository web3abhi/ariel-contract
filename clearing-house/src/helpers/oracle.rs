use crate::error::ContractError;

use crate::package::number::Number128;
use crate::package::types::{OracleGuardRails, OraclePriceData, OracleStatus};
use cosmwasm_std::Uint128;

use crate::helpers::amm;
use crate::states::market::Amm;

pub fn block_operation(
    a: &Amm,
    guard_rails: &OracleGuardRails,
    precomputed_mark_price: Option<Uint128>,
) -> Result<(bool, OraclePriceData), ContractError> {
    let OracleStatus {
        price_data: oracle_price_data,
        is_valid: oracle_is_valid,
        mark_too_divergent: is_oracle_mark_too_divergent,
        oracle_mark_spread_pct: _,
    } = get_oracle_status(
        a,
        guard_rails,
        precomputed_mark_price,
    )?;

    let block = !oracle_is_valid || is_oracle_mark_too_divergent;
    Ok((block, oracle_price_data))
}
 
pub fn get_oracle_status(
    a: &Amm,
    guard_rails: &OracleGuardRails,
    precomputed_mark_price: Option<Uint128>,
) -> Result<OracleStatus, ContractError> {
    let oracle_price_data = a.get_oracle_price()?;
    let oracle_is_valid = amm::is_oracle_valid(a, &oracle_price_data, &guard_rails)?;
    let oracle_mark_spread_pct =
        amm::calculate_oracle_mark_spread_pct(a, &oracle_price_data, precomputed_mark_price)?;
    let is_oracle_mark_too_divergent =
        amm::is_oracle_mark_too_divergent(oracle_mark_spread_pct, &guard_rails)?;

    Ok(OracleStatus {
        price_data: oracle_price_data,
        oracle_mark_spread_pct: Number128::new(oracle_mark_spread_pct) ,
        is_valid: oracle_is_valid,
        mark_too_divergent: is_oracle_mark_too_divergent,
    })
}
