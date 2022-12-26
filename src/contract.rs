#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128, Addr, WasmMsg, SubMsg, Reply, from_binary, BankMsg, CosmosMsg, Coin, Order,
};
use cw2::set_contract_version;

use cw20::{Balance, Cw20ReceiveMsg, Cw20CoinVerified, Cw20ExecuteMsg};

use crate::error::ContractError;
use crate::state::{State, STATE, OTCS, OTCInfo, UserInfo};
use crate::msg::{InstantiateMsg, QueryMsg, ExecuteMsg, ReceiveMsg, GetOTCsResponse, NewOTCResponse};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:otc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


macro_rules! cast {
    ($target: expr, $pat: path) => {
        {
            if let $pat(a) = $target { // #1
                a
            } else {
                panic!(
                    "mismatch variant when cast to {}", 
                    stringify!($pat)); // #2
            }
        }
    };
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let state = State { 
        active: true,
        index: 0,
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
        ExecuteMsg::Create(msg) => try_create_otc(
            deps,
            &info.sender,
            Balance::from(info.funds), 
            msg.ask_balance,    
            msg.ends_at,
            msg.user_info,
            msg.description
        ),

        ExecuteMsg::Swap { otc_id } => try_swap(
            deps, 
            &info.sender, 
            otc_id,
            Balance::from(info.funds),
            true
        ),
        
        ExecuteMsg::Receive(msg) => {
            execute_receive(deps, info, msg)
        }
    }
}


pub fn execute_receive(
    deps: DepsMut,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg : ReceiveMsg = from_binary(&wrapper.msg)?;

    let balance = Balance::Cw20(Cw20CoinVerified {
        address: info.sender,
        amount: wrapper.amount,
    });

    let api = deps.api;

    match msg {
        ReceiveMsg::Create(msg) => { 
            try_create_otc(
                deps, 
                &api.addr_validate(&wrapper.sender)?,
                balance,
                msg.ask_balance, 
                msg.ends_at,
                msg.user_info,
                msg.description
            )
        }
        ReceiveMsg::Swap { otc_id } => {
            try_swap(
                deps, 
                &api.addr_validate(&wrapper.sender)?, 
                otc_id,
                balance,
                false
            )
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


    while OTCS.has(deps.storage, config.index) {
        config.index = (config.index + 1) % 1000;    
    }

    OTCS.save(deps.storage, config.index, &new_otc)?;
    
    
    STATE.save(deps.storage, &config)?; 
   

    let data = NewOTCResponse {
        id: config.index,
        otc: new_otc
    };

    Ok(Response::new()
        .set_data(to_binary(&data).unwrap())
        .add_attribute("method", "create_new_otc")
    )

}



pub fn try_swap(
    deps: DepsMut,
    payer: &Addr,
    otc_id: u32,
    balance: Balance,
    native: bool,
    ) -> Result<Response, ContractError> {
    
    let otc_info = OTCS.load(deps.storage, otc_id)?;

    if otc_info.ask_native ^ native {
        return Err(ContractError::Std(
            StdError::GenericErr { 
                msg: "Wrong denomination".to_string() 
            }
        ));
    }

    let seller = deps.api.addr_humanize(&otc_info.seller)?;


    let payment_1 : CosmosMsg = if native {
        let mut casted =  cast!(balance, Balance::Native);
        let coin = casted.0.pop().unwrap();
        if casted.0.len() != 0 {
            return Err(ContractError::Std(
                StdError::GenericErr { 
                    msg: "Can't accept multiple denoms at time".to_string() 
                }
            ));
        }
        if coin.denom != otc_info.ask_denom.unwrap() {
            return Err(ContractError::Std(
                StdError::GenericErr { 
                    msg: "Wrong denomination".to_string() 
                }
            ));
        }
        if coin.amount < otc_info.ask_amount {
            return Err(ContractError::Std(
                StdError::GenericErr { 
                    msg: "Send amount is smaller than what being asked".to_string() 
                }
            ));
        }

        CosmosMsg::Bank(BankMsg::Send { to_address: seller.into_string(), amount: vec!(coin) })
        

    } else {
        let casted = cast!(balance, Balance::Cw20);

        if casted.address != otc_info.ask_address.unwrap() {
            return Err(ContractError::Std(
                StdError::GenericErr { 
                    msg: "Wrong cw20 token".to_string() 
                }
            ));
        }
        if casted.amount < otc_info.ask_amount {
            return Err(ContractError::Std(
                StdError::GenericErr { 
                    msg: "Send amount is smaller than what being asked".to_string() 
                }
            ));
        }

        CosmosMsg::Wasm(WasmMsg::Execute { 
            contract_addr: casted.address.to_string(), 
            msg: to_binary(&Cw20ExecuteMsg::Transfer { recipient: seller.to_string(), amount: casted.amount })?, 
            funds: vec!()
        })
        
    };


    let payment_2 : CosmosMsg = if otc_info.sell_native {
        CosmosMsg::Bank(BankMsg::Send { 
            to_address: payer.clone().into_string(), 
            amount: vec!(Coin { denom: otc_info.sell_denom.unwrap(), amount: otc_info.sell_amount }) 
        })
    } else {
        CosmosMsg::Wasm(WasmMsg::Execute { 
            contract_addr: otc_info.sell_address.unwrap().to_string(), 
            msg: to_binary(&Cw20ExecuteMsg::Transfer { 
                recipient: payer.to_string(), 
                amount: otc_info.sell_amount 
            })?, 
            funds: vec!()
        })
    };


    OTCS.remove(deps.storage, otc_id);
    

    Ok(Response::new()
        .add_messages(vec!(
            payment_1,
            payment_2
        ))
        .add_attribute("method", "swap")
    )
}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOtcs {} =>to_binary(&query_otcs(deps, env, msg)?)
    }
}



fn query_otcs(deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<GetOTCsResponse> {
    let result : StdResult<Vec<_>> = OTCS.range(
                        deps.storage, 
                        None, 
                        None, 
                        Order::Ascending
                    )
                    .collect();

    //OTCS.load(deps.storage, )
    Ok(GetOTCsResponse { otcs: result? })
}






#[cfg(test)]
mod tests {
    use crate::msg::{NewOTC, NewOTCResponse};

    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info,
    };
    use cosmwasm_std::{coins, from_binary};
    use cw_utils::NativeBalance;


    fn sell_native_ask_native(deps : DepsMut, count: u32) {

        let sell_amount = 5;
        let sell_denom = "token_1";

        let ask_amount = 10;
        let ask_denom = "token_2";

        let api = deps.api;


        let info = mock_info(
            "alice", 
            &coins(
            sell_amount.clone(), 
            sell_denom.clone()
        ));
        
        let msg = ExecuteMsg::Create(NewOTC {
            ask_balance: Balance::Native(NativeBalance(coins(ask_amount.clone(), ask_denom.clone()))),
            ends_at: 100,
            user_info: None,
            description: None,
        });

        let res = execute(deps, mock_env(), info, msg);
        match res {
            Ok(Response { 
                messages: _, 
                attributes, 
                events: _, 
                data, .. 
            }) => {
                let attr = attributes
                .iter()
                .find(|a| a.key == "method")
                .unwrap();

                assert!(attr.value == "create_new_otc");


                let res : NewOTCResponse = from_binary(&data.unwrap()).unwrap();

                api.debug(&format!("Left: {}, Right: {}", res.id, count));

                assert!(res.id == count);
                
                let info = res.otc;

                assert_eq!(info.ask_native, true);
                assert_eq!(info.ask_amount, Uint128::from(ask_amount));
                assert_eq!(info.ask_denom, Some(ask_denom.to_string()));

                assert_eq!(info.sell_native, true);
                assert_eq!(info.sell_amount, Uint128::from(sell_amount));
                assert_eq!(info.sell_denom, Some(sell_denom.to_string()));

                assert_eq!(info.ends_at, 100);

                // asserts other fields of OTCInfo struct
                assert_eq!(info.user_info, None);
                assert_eq!(info.description, None);

            }
            
            Err(ContractError::Std(StdError::GenericErr { msg })) => {
                panic!("Error {}", msg.as_str())
            },
            Err(ContractError::Unauthorized {  }) => {
                panic!("Unauthorized ")
            },
            _ => {
                panic!("Unknown error")
            }
        }
    }

    #[test]
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
    }

    #[test]
    fn can_create_native() {
        let mut deps = mock_dependencies();

        instantiate_contract(deps.as_mut());
        sell_native_ask_native(deps.as_mut(), 0);


    }
   


    /* #[test]
    fn can_query() {
        let mut deps = mock_dependencies_with_balance(&coins(2, "token"));

        instantiate_contract(deps.as_mut());

        let res = query(deps.as_ref(), mock_env(), QueryMsg::Hello {}).unwrap();
        let value: HelloResponse = from_binary(&res).unwrap();
        assert_eq!("Hello, Archway!", value.msg);
    }
    */

    fn instantiate_contract(deps: DepsMut) -> Response {
        let msg = InstantiateMsg {};
        let info = mock_info("creator", &coins(1000, "token"));
        instantiate(deps, mock_env(), info, msg).unwrap()
    } 
}
