#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::error::ContractError;
    use crate::msg::{NewOTC, NewOTCResponse, ExecuteMsg, InstantiateMsg, QueryMsg, GetOTCsResponse};

    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, mock_dependencies_with_balances, 
    };
    use cosmwasm_std::{coins, from_binary, DepsMut, Response, Uint128,  Coin, Deps, Api};
    use cw20::Balance;
    use cw_utils::{NativeBalance, Expiration};


    fn sell_native_ask_native(deps : DepsMut, count: u32) {

        let sell_amount = 5;
        let sell_denom = "token_1";

        let ask_amount = 10;
        let ask_denom = "token_2";

        //let api = deps.api;
        

        let alice_balance_before = deps.as_ref().querier.query_balance("alice", "token_1").unwrap();
        assert_eq!(alice_balance_before.amount, Uint128::from(5 as u8));

        let info = mock_info(
            "alice", 
            &coins(
            sell_amount.clone(), 
            sell_denom.clone()
        ));
        
        let msg = ExecuteMsg::Create(NewOTC {
            ask_balance: Balance::Native(NativeBalance(coins(ask_amount.clone(), ask_denom.clone()))),
            ends_at: None,
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

                assert!(res.id == count);
                
                let info = res.otc;

                assert_eq!(info.ask_native, true);
                assert_eq!(info.ask_amount, Uint128::from(ask_amount));
                assert_eq!(info.ask_denom, Some(ask_denom.to_string()));

                assert_eq!(info.sell_native, true);
                assert_eq!(info.sell_amount, Uint128::from(sell_amount));
                assert_eq!(info.sell_denom, Some(sell_denom.to_string()));

                assert_eq!(info.expires, Expiration::Never {} );

                // asserts other fields of OTCInfo struct
                assert_eq!(info.user_info, None);
                assert_eq!(info.description, None);

            }
            _ => {
                panic!("Unknown error")
            }
        }
    }


    fn query_otcs(deps: Deps) -> GetOTCsResponse {
        let res = query(deps, mock_env(), QueryMsg::GetOtcs {
            include_expired: None,
            limit: None,
            start_after: None,
        }).unwrap();
        let value: GetOTCsResponse = from_binary(&res).unwrap();
        value
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
        assert_eq!("admin", owner);
    }

    #[test]
    fn can_create_and_swap_native() {
        let mut deps = mock_dependencies_with_balances(&[
            ("alice", &coins(5, "token_1")),
            ("bob", &coins(10, "token_2")),
        ]);


        let count = 0;

        instantiate_contract(deps.as_mut());
        
        
        assert_eq!(query_otcs(deps.as_ref()).otcs.len(), 0);


        sell_native_ask_native(deps.as_mut(), count.clone());


        let otcs = query_otcs(deps.as_ref()).otcs;
        assert_eq!(otcs.len(), 1);

        let (id, otc) = &otcs[0];
        assert_eq!(id, &count);

        assert!(
            otc.ask_native &&
            otc.ask_amount == Uint128::from(10 as u8) &&
            deps.api.addr_humanize(&otc.seller).unwrap() == "alice",
        );


        let same_person_info = mock_info("alice", &coins(10, "token_2"));
        let smaller_amount_info = mock_info("bob", &coins(1, "token_2"));
        let wrong_denom_info = mock_info("bob", &coins(10, "token_3"));
        let multiple_tokens_info = mock_info("bob", 
            &vec!(
                Coin { 
                    amount: Uint128::from(10 as u8), 
                    denom: "token_2".to_string() 
                }, 
                Coin { 
                    amount: Uint128::from(10 as u8), 
                    denom: "token_3".to_string() 
                }
            )
        );
        //let bigger_amount_info = mock_info("bob", &coins(100, "token_2"));
        let right_info = mock_info("bob", &coins(10, "token_2"));

        let msg = ExecuteMsg::Swap { otc_id: count.clone() };

        let res = execute(deps.as_mut(), mock_env(), same_person_info, msg.clone()).unwrap_err();
        assert_eq!(res.to_string(), "Can't swap with yourself");


        let res = execute(deps.as_mut(), mock_env(), smaller_amount_info, msg.clone()).unwrap_err();
        assert_eq!(res.to_string(), "Sent amount is smaller than what being asked");
     

        let res = execute(deps.as_mut(), mock_env(), wrong_denom_info, msg.clone()).unwrap_err();
        assert_eq!(res.to_string(), ContractError::WrongDenom {}.to_string());
       

        let res = execute(deps.as_mut(), mock_env(), multiple_tokens_info, msg.clone()).unwrap_err();
        assert_eq!(res.to_string(), ContractError::TooManyDenoms {}.to_string());


        let res = execute(deps.as_mut(), mock_env(), right_info, msg.clone()).unwrap();
        
        let attr = res.attributes
                .iter()
                .find(|a| a.key == "method")
                .unwrap();

        assert!(attr.value == "swap");


        /* Mock querier doesn't work. Figure a way around it
        
        let querier = deps.as_ref().querier;
        let alice_balance = querier.query_all_balances("alice").unwrap()[0].clone();
        let bob_balance = querier.query_all_balances("bob").unwrap()[0].clone();

        api.debug(&format!("Alice balance: {:?} {}", alice_balance.amount, alice_balance.denom));
        api.debug(&format!("Bob balance: {:?} {}", bob_balance.amount, bob_balance.denom));

        assert_eq!(alice_balance.amount, Uint128::from(0 as u8));
        assert_eq!(bob_balance.amount, Uint128::from(10 as u8)); 
        */

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
        let info = mock_info("admin", &[]);
        instantiate(deps, mock_env(), info, msg).unwrap()
    } 
}
