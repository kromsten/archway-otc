use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw20::{Balance, Cw20ReceiveMsg};

use crate::state::UserInfo;



#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub otc_code_hash: u64
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct NewOTC {
    ask_balance: Balance, 

    // seconds since epoch
    ends_at: u64,

    // optional user info
    user_info: Option<UserInfo>,

    // optional description
    description: Option<String>
},




#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Create(NewOTC),

    Swap {
        otc_id: u32
    },

    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    
    Create(NewOTC),

    Swap {
        otc_id: u32
    }
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