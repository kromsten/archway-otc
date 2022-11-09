use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw20::Denom;

use crate::otc_msg::UserInfo;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub otc_code_hash: u64
}



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    NewOTC {
        /// String label for the otc
        label: String,

        // Denomination of currency being sold
        sell_denom: Denom,

        // Denomination of currency being asked
        ask_denom: Denom,

        /// amount of tokens being sold
        sell_amount: Uint128,

        /// (minimum) amount of tokens being sold
        ask_amount: Uint128,

        // seconds since epoch
        ends_at: u64,

        // optional user info
        user_info: Option<UserInfo>,

        // optional description
        description: Option<String>
    },
}



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Hello {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HelloResponse {
    pub msg: String,
}
