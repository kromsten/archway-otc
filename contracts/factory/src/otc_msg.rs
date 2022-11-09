
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw20::Denom;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UserInfo {
    pub user: String,
    pub account_type: Option<String>,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OTCInitMsg {
    /// auction index with the factory
    pub index: u64,

    /// otc creator
    pub seller: Addr,

    // Denomination of currency being sold
    pub sell_denom: Denom,

    // Denomination of currency being asked
    pub ask_denom: Denom,

    /// amount of tokens being sold
    pub sell_amount: Uint128,

    /// (minimum) amount of tokens being sold
    pub ask_amount: Uint128,

    // seconds since epoch
    pub ends_at: u64,

  

    // optional user info
    pub user_info: Option<UserInfo>,

    // optional description
    pub description: Option<String>,
}



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OTCInitResponse {
    pub otc_address: Addr,

}

