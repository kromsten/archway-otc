#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128, Addr, WasmMsg, SubMsg, Reply, from_binary,
};
use cw2::set_contract_version;

use::cw20::Denom;

use crate::error::ContractError;
use crate::msg::{HelloResponse, InstantiateMsg, QueryMsg, ExecuteMsg};
use crate::state::{State, STATE, OTCS, OTCInfo};
use crate::otc_msg::{UserInfo, OTCInitMsg, OTCInitResponse};

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
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::NewOTC {
            label, 
            sell_denom, 
            sell_amount, 
            ask_denom, 
            ask_amount, 
            ends_at, 
            user_info, 
            description } => try_create_otc(
                deps,
                env,
                info.sender,
                label, 
                sell_denom, 
                ask_denom, 
                sell_amount,
                ask_amount,
                ends_at,
                user_info,
                description
            ),
    }
}


pub fn try_create_otc(
    deps: DepsMut,
    env: Env,
    seller: Addr,
    label: String,
    sell_denom: Denom,
    ask_denom: Denom,
    sell_amount : Uint128,
    ask_amount: Uint128,
    ends_at: u64,
    user_info: Option<UserInfo>,
    description: Option<String>,
    ) -> Result<Response, ContractError> {
    

    let config = STATE.load(deps.storage)?;


    if !config.active {
        return Err(ContractError::Std(
            StdError::GenericErr { 
                msg: "The factory has been stopped.  No new otc can be created".to_string() 
            }
        ));
    }

    let init_msg_content = OTCInitMsg {
        index: config.index,
        seller,
        sell_denom,
        ask_denom,
        sell_amount,
        ask_amount,
        ends_at,
        user_info,
        description,
    };
    
    let instantiate_message = WasmMsg::Instantiate {
        admin: Some(env.contract.address.into_string()),
        code_id: config.otc_code_hash,
        msg: to_binary(&init_msg_content)?,
        funds: vec![],
        label: label,
    };


    let submessage = SubMsg::reply_on_success(instantiate_message, config.index.clone());


    Ok(Response::new()
        .add_submessage(submessage)
        .add_attribute("method", "create_new_otc")
    )

}





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
        config.index,
        &OTCInfo {}
    )?;


    config.index += 1;
    STATE.save(deps.storage, &config)?;

    
    Ok(Response::new())
}




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
