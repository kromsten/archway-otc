#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128, Addr, WasmMsg, SubMsg, Reply, from_binary,
};
use cw2::set_contract_version;

use cw20::{Balance, Cw20ReceiveMsg, Cw20CoinVerified};

use crate::error::ContractError;
use crate::msg::{HelloResponse, InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{State, STATE, OTCS, OTCInfo, UserInfo};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:otc_factory";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let state = State { 
        active: true,
        index: 0,
        otc_code_hash: msg.otc_code_hash,
        admin: deps.api.addr_canonicalize(info.sender.as_str())?
    };

    STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}






#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::NewOTC {
            ask_balance, 
            ends_at, 
            user_info, 
            description 
        } => try_create_otc(
            deps,
            &info.sender,
            Balance::from(info.funds), 
            ask_balance,    
            ends_at,
            user_info,
            description
        ),
        
        ExecuteMsg::Receive(msg) => execute_receive(deps, info, msg)
    }
}


pub fn execute_receive(
    deps: DepsMut,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg : ExecuteMsg = from_binary(&wrapper.msg)?;

    let sell_balance = Balance::Cw20(Cw20CoinVerified {
        address: info.sender,
        amount: wrapper.amount,
    });

    let api = deps.api;

    match msg {
        ExecuteMsg::NewOTC { 
            ask_balance, 
            ends_at, 
            user_info, 
            description 
        } => { 
            try_create_otc(
                deps, 
                &api.addr_validate(&wrapper.sender)?,
                sell_balance,
                ask_balance, 
                ends_at,
                user_info,
                description
            )
        }
        _ => {
            return Err(ContractError::Std(
                StdError::GenericErr { 
                    msg: "Unknown Receive message ".to_string() 
                }
            ));
        }
    }


    
}

pub fn try_create_otc(
    deps: DepsMut,
    seller: &Addr,
    sell_balance: Balance,
    ask_balance: Balance,
    ends_at: u64,
    user_info: Option<UserInfo>,
    description: Option<String>,
    ) -> Result<Response, ContractError> {
    

    let mut config = STATE.load(deps.storage)?;


    if !config.active {
        return Err(ContractError::Std(
            StdError::GenericErr { 
                msg: "The factory has been stopped.  No new otc can be created".to_string() 
            }
        ));
    }

    let mut new_otc = OTCInfo {
        seller: deps.api.addr_canonicalize(seller.as_str())?,
        sell_native: false,
        sell_amount: Uint128::from(0 as u8),
        sell_denom: None,
        sell_address: None,
        ask_native: false,
        ask_amount: Uint128::from(0 as u8),
        ask_denom: None,
        ask_address: None,
        ends_at,
        user_info,
        description
    };


    match sell_balance {
        Balance::Native(mut balance) => {

            let coin = balance.0.pop().unwrap();

            if balance.0.len() != 0 {
                return Err(ContractError::Std(
                    StdError::GenericErr { 
                        msg: "Cannot create an otc with mupltiple denoms".to_string() 
                    }
                ));
            }

            new_otc.sell_native = true;
            new_otc.sell_amount = coin.amount;
            new_otc.sell_denom = Some(coin.denom);
        },
        Balance::Cw20(token) => {
            new_otc.sell_native = false;
            new_otc.sell_amount = token.amount;
            new_otc.sell_address = Some(token.address);
        }
    };


    match ask_balance {
        Balance::Native(mut balance) => {

            let coin = balance.0.pop().unwrap();

            if balance.0.len() != 0 {
                return Err(ContractError::Std(
                    StdError::GenericErr { 
                        msg: "Cannot create an otc with mupltiple denoms".to_string() 
                    }
                ));
            }

            new_otc.ask_native = true;
            new_otc.ask_amount = coin.amount;
            new_otc.ask_denom = Some(coin.denom);
        },
        Balance::Cw20(token) => {
            new_otc.ask_native = false;
            new_otc.ask_amount = token.amount;
            new_otc.ask_address = Some(token.address);
        }
    };

    

    OTCS.save(deps.storage, &config.index.to_be_bytes(), &new_otc)?;
    
    config.index += 1;
    STATE.save(deps.storage, &config)?; 
   

    Ok(Response::new()
        .add_attribute("method", "create_new_otc")
    )

}




/* 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {

    let config = STATE.load(deps.storage)?;
    let index = config.index.clone();

    if msg.id == index {
        handle_instantiate_reply(deps, msg, config)
    } else {
        Err(ContractError::Std(StdError::generic_err(format!("Unknown reply id: {}", msg.id))))
    }
}

fn handle_instantiate_reply(deps: DepsMut, msg: Reply, mut config: State) -> Result<Response, ContractError> {
    // Handle the msg data and save the contract address
    let data = msg.result.unwrap().data.unwrap();
    

    let res: OTCInitResponse = from_binary(&data).map_err(|_| {
        StdError::parse_err("MsgInstantiateContractResponse", "failed to parse data")
    })?;


    OTCS.save(
        deps.storage, 
        &config.index.to_le_bytes(),
        &OTCInfo {}
    )?;


    config.index += 1 ;
    STATE.save(deps.storage, &config)?;


    Ok(Response::new())
}


 */

pub fn try_execute(_deps: DepsMut) -> Result<Response, ContractError> {
    Err(ContractError::Std(StdError::generic_err("Not implemented")))
    // TODO: Ok(Response::new().add_attribute("method", "try_execute"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Hello {} => to_binary(&hello_world()?),
    }
}

fn hello_world() -> StdResult<HelloResponse> {
    Ok(HelloResponse {
        msg: String::from("Hello, Archway!"),
    })
}




#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
    };
    use cosmwasm_std::{coins, from_binary};

    /* #[test]
    fn can_instantiate() {
        let mut deps = mock_dependencies();

        let res = instantiate_contract(deps.as_mut());
        assert_eq!(0, res.messages.len());

        let owner = &res
            .attributes
            .iter()
            .find(|a| a.key == "owner")
            .unwrap()
            .value;
        assert_eq!("creator", owner);
    } */

    /* #[test]
    fn can_execute() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        instantiate_contract(deps.as_mut());

        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Dummy {};

        // TODO: fix this test when execute() is implemented
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res {
            Err(ContractError::Std(StdError::GenericErr { msg })) => {
                assert_eq!("Not implemented", msg)
            }
            _ => panic!("Must return not implemented error"),
        }
    }
    */


    /* #[test]
    fn can_query() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        instantiate_contract(deps.as_mut());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Hello {}).unwrap();
        let value: HelloResponse = from_binary(&res).unwrap();
        assert_eq!("Hello, Archway!", value.msg);
    }

    fn instantiate_contract(deps: DepsMut) -> Response {
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "token"));
        instantiate(deps, mock_env(), info, msg).unwrap()
    } */
}
