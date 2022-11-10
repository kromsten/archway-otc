#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128, Addr, WasmMsg, SubMsg, Reply, from_binary, BankMsg, CosmosMsg, Coin,
};
use cw2::set_contract_version;

use cw20::{Balance, Cw20ReceiveMsg, Cw20CoinVerified, Cw20ExecuteMsg};

use crate::error::ContractError;
use crate::state::{State, STATE, OTCS, OTCInfo, UserInfo};
use crate::msg::{HelloResponse, InstantiateMsg, QueryMsg, ExecuteMsg, ReceiveMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:otc_factory";
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


    OTCS.save(deps.storage, &config.index.to_be_bytes(), &new_otc)?;
    
    config.index += 1;
    STATE.save(deps.storage, &config)?; 
   

    Ok(Response::new()
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
    
    let otc_info = OTCS.load(deps.storage, &otc_id.to_be_bytes())?;

    if otc_info.ask_native ^ native {
        return Err(ContractError::Std(
            StdError::GenericErr { 
                msg: "Wrong denomination".to_string() 
            }
        ));
    }

    let seller = deps.api.addr_humanize(&otc_info.seller)?;


    let payment_1 : CosmosMsg = if native {
        let casted =  cast!(balance, Balance::Native);
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
            to_address: payer.into_string(), 
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


    let config = STATE.load(deps.storage)?;


    Ok(Response::new()
        .add_messages(vec!(
            payment_1,
            payment_2
        ))
        .add_attribute("method", "swap")
    )
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
