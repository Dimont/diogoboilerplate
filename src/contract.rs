#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{ 
    from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Addr, BankMsg, SubMsg, WasmMsg 
};
use cw2::set_contract_version;
use cw20::{
    Balance, Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg, Cw20ReceiveMsg
};

use crate::error::ContractError;
use crate::msg::{InstantiateMsg, ExecuteMsg, QueryMsg, ListResponse, DetailsResponse, CreateMsg, ReceiveMsg};
use crate::state::{ 
    Escrow, ESCROWS, all_escrow_ids, GenericBalance
 };

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:diogoboilerplate";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    /*let state = State {
        count: msg.count,
        owner: info.sender.clone(),
    };*/
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    //STATE.save(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        /*.add_attribute("owner", info.sender)
        .add_attribute("count", msg.count.to_string())*/
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateEscrow (msg) => try_create_escrow(deps, msg, Balance::from(info.funds), &info.sender),

        ExecuteMsg::SetRecipient { id, recipient } => try_set_recipient(deps, env, info, id, recipient),

        ExecuteMsg::Approve { id } => try_approve(deps, env, info, id),

        ExecuteMsg::Refund { id } => try_refund(deps, env, info, id),

        ExecuteMsg::TopUp { id } => try_top_up(deps, id, Balance::from(info.funds)),

        ExecuteMsg::Receive(msg) => try_receive(deps, info, msg),
    }
}

pub fn try_create_escrow(
    deps: DepsMut,
    msg: CreateMsg,
    balance: Balance,
    sender: &Addr,
) -> Result<Response, ContractError> {
    if balance.is_empty(){
        return Err(ContractError::EmptyBalance{});
    }

    let mut cw20_whitelist = msg.addr_whitelist(deps.api)?;

    let escrow_balance = match balance {
        Balance::Native(balance) => GenericBalance {
            native: balance.0,
            cw20: vec![],
        },
        Balance::Cw20(token) => {
            //make sure the token sent is on the whitelist by default
            if !cw20_whitelist.iter().any(|t| t == &token.address) {
                cw20_whitelist.push(token.address.clone())
            }
            GenericBalance {
                native: vec![],
                cw20: vec![token],
            }
        }
    };

    let recipient: Option<Addr> = msg
        .recipient
        .and_then(|addr| deps.api.addr_validate(&addr).ok());

    let escrow = Escrow {
        arbiter: deps.api.addr_validate(&msg.arbiter)?,
        recipient,
        source: sender.clone(),
        title: msg.title,
        description: msg.description,
        end_height: msg.end_height,
        end_time: msg.end_time,
        balance: escrow_balance,
        cw20_whitelist,
    };

    // try to store it, fail if the id was already in use
    ESCROWS.update(deps.storage, &msg.id, |existing| match existing {
        None => Ok(escrow),
        Some(_) => Err(ContractError::AlreadyInUse {}),
    })?;

    let res = Response::new().add_attributes(vec![("action", "create_escrow"), ("id", msg.id.as_str())]);
    Ok(res)
}

pub fn try_set_recipient(
    deps: DepsMut, _env: Env, info: MessageInfo, id: String, recipient: String
)-> Result<Response, ContractError> {
    let mut escrow = ESCROWS.load(deps.storage, &id)?;
    if info.sender != escrow.arbiter {
        return Err(ContractError::Unauthorized {});
    }

    let recipient = deps.api.addr_validate(recipient.as_str())?;
    escrow.recipient = Some(recipient.clone());
    ESCROWS.save(deps.storage, &id, &escrow)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "set_recipient"),
        ("id", id.as_str()),
        ("recipient", recipient.as_str()),
    ]))
}

pub fn try_approve(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    
    let escrow = ESCROWS.load(deps.storage, &id)?;
    if info.sender != escrow.arbiter {
        return Err(ContractError::Unauthorized {});
    }
    if escrow.is_expired(&env) {
        return Err(ContractError::Expired {});
    }
    
    let recipient = escrow.recipient.ok_or(ContractError::RecipientNotSet {})?;
    
    // we delete the escrow
    ESCROWS.remove(deps.storage, &id);
    
    // send all tokens out
    let messages: Vec<SubMsg> = send_tokens(&recipient, &escrow.balance)?;
    Ok(Response::new()
        .add_attribute("action", "approve")
        .add_attribute("id", id)
        .add_attribute("to", recipient)
        .add_submessages(messages))
}

pub fn try_refund(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    id: String,
) -> Result<Response, ContractError> {
    /*
    // this fails is no escrow there
    let escrow = ESCROWS.load(deps.storage, &id)?;

    // the arbiter can send anytime OR anyone can send after expiration
    if !escrow.is_expired(&env) && info.sender != escrow.arbiter {
        Err(ContractError::Unauthorized {})
    } else {
        // we delete the escrow
        ESCROWS.remove(deps.storage, &id);

        // send all tokens out
        let messages = send_tokens(&escrow.source, &escrow.balance)?;

        Ok(Response::new()
            .add_attribute("action", "refund")
            .add_attribute("id", id)
            .add_attribute("to", escrow.source)
            .add_submessages(messages))
    }*/
    Ok(Response::default())
}

pub fn try_top_up(
    deps: DepsMut,
    id: String,
    balance: Balance,
) -> Result<Response, ContractError> {
    /*
    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }
    // this fails if no escrow there
    let mut escrow = ESCROWS.load(deps.storage, &id)?;

    if let Balance::Cw20(token) = &balance {
        // ensure the token is on the whitelist
        if !escrow.cw20_whitelist.iter().any(|t| t == &token.address) {
            return Err(ContractError::NotInWhitelist {});
        }
    };

    escrow.balance.add_tokens(balance);

    // and save
    ESCROWS.save(deps.storage, &id, &escrow)?;

    let res = Response::new().add_attributes(vec![("action", "top_up"), ("id", id.as_str())]);
    Ok(res)*/
    Ok(Response::default())
}

pub fn try_receive(
    deps: DepsMut,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    /*
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let balance = Balance::Cw20(Cw20CoinVerified {
        address: info.sender,
        amount: wrapper.amount,
    });
    let api = deps.api;
    match msg {
        ReceiveMsg::Create(msg) => {
            execute_create(deps, msg, balance, &api.addr_validate(&wrapper.sender)?)
        }
        ReceiveMsg::TopUp { id } => execute_top_up(deps, id, balance),
    }*/
    Ok(Response::default())
}


// study more deeply
fn send_tokens(to: &Addr, balance: &GenericBalance) -> StdResult<Vec<SubMsg>> {
    let native_balance = &balance.native;
    let mut msgs: Vec<SubMsg> = if native_balance.is_empty() {
        vec![]
    } else {
        vec![SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: native_balance.to_vec(),
        })]
    };

    let cw20_balance = &balance.cw20;
    let cw20_msgs: StdResult<Vec<_>> = cw20_balance
        .iter()
        .map(|c| {
            let msg = Cw20ExecuteMsg::Transfer {
                recipient: to.into(),
                amount: c.amount,
            };
            let exec = SubMsg::new(WasmMsg::Execute {
                contract_addr: c.address.to_string(),
                msg: to_binary(&msg)?,
                funds: vec![],
            });
            Ok(exec)
        })
        .collect();
    msgs.append(&mut cw20_msgs?);
    Ok(msgs)
}

/*pub fn try_increment(deps: DepsMut) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        state.count += 1;
        Ok(state)
    })?;

    Ok(Response::new().add_attribute("method", "try_increment"))
}*/

/*pub fn try_reset(deps: DepsMut, info: MessageInfo, count: i32) -> Result<Response, ContractError> {
    STATE.update(deps.storage, |mut state| -> Result<_, ContractError> {
        if info.sender != state.owner {
            return Err(ContractError::Unauthorized {});
        }
        state.count = count;
        Ok(state)
    })?;
    Ok(Response::new().add_attribute("method", "reset"))
}*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::List {} => to_binary(&query_list(deps)?),
        QueryMsg::Details { id } => to_binary(&query_details(deps, id)?),
    }
}

fn query_list(deps: Deps) -> StdResult<ListResponse> {
    Ok(
        ListResponse {
            escrows: all_escrow_ids(deps.storage)?,
        }
    )
}

fn query_details(deps: Deps, id: String) -> StdResult<DetailsResponse> {
    let escrow = ESCROWS.load(deps.storage, &id)?;
    let cw20_whitelist = escrow.human_whitelist();

    let native_balance = escrow.balance.native;

    let cw20_balance: StdResult<Vec<_>> = escrow
        .balance.cw20.into_iter()
        .map(|token| {
            Ok(Cw20Coin {
                address: token.address.into(),
                amount: token.amount,
            })
        }).collect();
    
    let recipient = escrow.recipient.map(|addr| addr.into_string());

    let details = DetailsResponse {
        id,
        arbiter: escrow.arbiter.into(),
        recipient,
        source: escrow.source.into(),
        title: escrow.title,
        description: escrow.description,
        end_height: escrow.end_height,
        end_time: escrow.end_time,
        native_balance,
        cw20_balance: cw20_balance?,
        cw20_whitelist,
    };

    Ok(details)
}

/*fn query_count(deps: Deps) -> StdResult<GetCountResponse> {
    let state = STATE.load(deps.storage)?;
    Ok(GetCountResponse { count: state.count })
}*/

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, coin, coins, CosmosMsg, StdError, Uint128};
    use crate::msg::ExecuteMsg::TopUp;

    #[test]
    fn instantiate_test(){
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&String::from("anyone"), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn instantiate_and_create_escrow_and_query_details_test(){
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&String::from("anyone"), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "some_id".to_string(),
            arbiter: String::from("some_arbiter"),
            recipient: Some(String::from("some_recipient")),
            title: "some_title".to_string(),
            end_time: None,
            end_height: Some(123456),
            cw20_whitelist: None,
            description: "some_description".to_string(),
        };
        let sender = String::from("source");
        let balance = coins(100, "tokens");
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::CreateEscrow(create.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create_escrow"), res.attributes[0]);

        // ensure the details is what we expect
        let details = query_details(deps.as_ref(), "some_id".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "some_id".to_string(),
                arbiter: String::from("some_arbiter"),
                recipient: Some(String::from("some_recipient")),
                source: String::from("source"),
                title: "some_title".to_string(),
                description: "some_description".to_string(),
                end_height: Some(123456),
                end_time: None,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
            }
        );
    }

    #[test]
    fn instantiate_and_create_escrow_and_approve_and_approve_again_test() {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&String::from("anyone"), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "some_id".to_string(),
            arbiter: String::from("some_arbiter"),
            recipient: Some(String::from("some_recipient")),
            title: "some_title".to_string(),
            end_time: None,
            end_height: Some(123456),
            cw20_whitelist: None,
            description: "some_description".to_string(),
        };
        let sender = String::from("source");
        let balance = coins(100, "tokens");
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::CreateEscrow(create.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create_escrow"), res.attributes[0]);

        // ensure the details is what we expect
        let details = query_details(deps.as_ref(), "some_id".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "some_id".to_string(),
                arbiter: String::from("some_arbiter"),
                recipient: Some(String::from("some_recipient")),
                source: String::from("source"),
                title: "some_title".to_string(),
                description: "some_description".to_string(),
                end_height: Some(123456),
                end_time: None,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
            }
        );

        // approve it
        let id = create.id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Approve { id }).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(("action", "approve"), res.attributes[0]);
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: create.recipient.unwrap(),
                amount: balance,
            }))
        );

        // second attempt fails (not found)
        let id = create.id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Approve { id }).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));
    }

    #[test]
    fn create_escrow_and_set_recipient_test(){
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&String::from("anyone"), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // create an escrow
        let create = CreateMsg {
            id: "test_id".to_string(),
            arbiter: String::from("test_arbiter"),
            recipient: None,
            title: "test_title".to_string(),
            end_time: None,
            end_height: Some(123456),
            cw20_whitelist: None,
            description: "test_description".to_string(),
        };
        let sender = String::from("test_source");
        let balance = coins(100, "tokens");
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::CreateEscrow(create.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create_escrow"), res.attributes[0]);

        // ensure the details is what we expect
        let details = query_details(deps.as_ref(), "test_id".to_string()).unwrap();
        assert_eq!(
            details,
            DetailsResponse {
                id: "test_id".to_string(),
                arbiter: String::from("test_arbiter"),
                recipient: None,
                source: String::from("test_source"),
                title: "test_title".to_string(),
                description: "test_description".to_string(),
                end_height: Some(123456),
                end_time: None,
                native_balance: balance.clone(),
                cw20_balance: vec![],
                cw20_whitelist: vec![],
            }
        );

        // approve it, should fail as we have not set recipient
        let id = create.id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Approve { id });
        match res {
            Err(ContractError::RecipientNotSet {}) => {}
            _ => panic!("Expect recipient not set error"),
        }

        // test someone that is not arbiter setting recipient should fail
        let msg = ExecuteMsg::SetRecipient {
            id: create.id.clone(),
            recipient: "recp".to_string(),
        };
        let info = mock_info("someoneelse", &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Expect unauthorized error"),
        }

        // test setting recipient valid
        let msg = ExecuteMsg::SetRecipient {
            id: create.id.clone(),
            recipient: "recp".to_string(),
        };
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages.len(), 0);
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "set_recipient"),
                attr("id", create.id.as_str()),
                attr("recipient", "recp")
            ]
        );

        // approve it, should now work with recp
        let id = create.id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Approve { id }).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(("action", "approve"), res.attributes[0]);
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: "recp".to_string(),
                amount: balance,
            }))
        );
    }

    #[test]
    fn add_native_tokens_proper() {
        let mut tokens = GenericBalance::default();
        tokens.add_tokens(Balance::from(vec![coin(123, "atom"), coin(789, "eth")]));
        tokens.add_tokens(Balance::from(vec![coin(456, "atom"), coin(12, "btc")]));
        // necessary ?
        assert_eq!(
            tokens.native,
            vec![coin(579, "atom"), coin(789, "eth"), coin(12, "btc")]
        );
    }
    

    #[test]
    fn add_cw_tokens_proper() {
        let mut tokens = GenericBalance::default();
        let test_token = Addr::unchecked("test_token");
        let test2_token = Addr::unchecked("test2_token");
        tokens.add_tokens(Balance::Cw20(Cw20CoinVerified {
            address: test_token.clone(),
            amount: Uint128::new(12345),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20CoinVerified {
            address: test2_token.clone(),
            amount: Uint128::new(777),
        }));
        tokens.add_tokens(Balance::Cw20(Cw20CoinVerified {
            address: test_token.clone(),
            amount: Uint128::new(23400),
        }));
        assert_eq!(
            tokens.cw20,
            vec![
                Cw20CoinVerified {
                    address: test_token,
                    amount: Uint128::new(35745),
                },
                Cw20CoinVerified {
                    address: test2_token,
                    amount: Uint128::new(777),
                }
            ]
        );
    }

    #[test]
    fn top_up_mixed_tokens() {
        // instantiate an empty contract &&  accept only certain tokens
        // create an escrow with 2 native tokens &&  top it up with 2 more native tokens
        // top up with one foreign token
        // top with a foreign token not on the whitelist && top up with one foreign token
        // top up with second foreign token
        // approve it  &&  first message releases all native coins
        // second one release bar cw20 token  &&  third one release foo cw20 token
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info(&String::from("anyone"), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // only accept these tokens
        let whitelist = vec![String::from("bar_token"), String::from("foo_token")];

        // create an escrow with 2 native tokens
        let create = CreateMsg {
            id: "foobar".to_string(),
            arbiter: String::from("arbitrate"),
            recipient: Some(String::from("recd")),
            title: "some_title".to_string(),
            end_time: None,
            end_height: None,
            cw20_whitelist: Some(whitelist),
            description: "some_description".to_string(),
        };
        let sender = String::from("source");
        let balance = vec![coin(100, "fee"), coin(200, "stake")];
        let info = mock_info(&sender, &balance);
        let msg = ExecuteMsg::CreateEscrow(create.clone());
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create_escrow"), res.attributes[0]);

        // top it up with 2 more native tokens
        let extra_native = vec![coin(250, "random"), coin(300, "stake")];
        let info = mock_info(&sender, &extra_native);
        let top_up = ExecuteMsg::TopUp {
            id: create.id.clone(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "top_up"), res.attributes[0]);

        // top up with one foreign token
        let bar_token = String::from("bar_token");
        let base = TopUp {
            id: create.id.clone(),
        };
        let top_up = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: String::from("random"),
            amount: Uint128::new(7890),
            msg: to_binary(&base).unwrap(),
        });
        let info = mock_info(&bar_token, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "top_up"), res.attributes[0]);

        // top with a foreign token not on the whitelist
        // top up with one foreign token
        let baz_token = String::from("baz_token");
        let base = TopUp {
            id: create.id.clone(),
        };
        let top_up = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: String::from("random"),
            amount: Uint128::new(7890),
            msg: to_binary(&base).unwrap(),
        });
        let info = mock_info(&baz_token, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, top_up).unwrap_err();
        assert_eq!(err, ContractError::NotInWhitelist {});

        // top up with second foreign token
        let foo_token = String::from("foo_token");
        let base = TopUp {
            id: create.id.clone(),
        };
        let top_up = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: String::from("random"),
            amount: Uint128::new(888),
            msg: to_binary(&base).unwrap(),
        });
        let info = mock_info(&foo_token, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, top_up).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "top_up"), res.attributes[0]);

        // approve it
        let id = create.id.clone();
        let info = mock_info(&create.arbiter, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Approve { id }).unwrap();
        assert_eq!(("action", "approve"), res.attributes[0]);
        assert_eq!(3, res.messages.len());

        // first message releases all native coins
        assert_eq!(
            res.messages[0],
            SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: create.recipient.clone().unwrap(),
                amount: vec![coin(100, "fee"), coin(500, "stake"), coin(250, "random")],
            }))
        );

        // second one release bar cw20 token
        let send_msg = Cw20ExecuteMsg::Transfer {
            recipient: create.recipient.clone().unwrap(),
            amount: Uint128::new(7890),
        };
        assert_eq!(
            res.messages[1],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: bar_token,
                msg: to_binary(&send_msg).unwrap(),
                funds: vec![]
            }))
        );

        // third one release foo cw20 token
        let send_msg = Cw20ExecuteMsg::Transfer {
            recipient: create.recipient.unwrap(),
            amount: Uint128::new(888),
        };
        assert_eq!(
            res.messages[2],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: foo_token,
                msg: to_binary(&send_msg).unwrap(),
                funds: vec![]
            }))
        );
    }


    /*#[test]
    fn increment() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Increment {};
        let _res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // should increase counter by 1
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }*/

    /*#[test]
    fn reset() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg { count: 17 };
        let info = mock_info("creator", &coins(2, "token"));
        let _res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // beneficiary can release it
        let unauth_info = mock_info("anyone", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let res = execute(deps.as_mut(), mock_env(), unauth_info, msg);
        match res {
            Err(ContractError::Unauthorized {}) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_info = mock_info("creator", &coins(2, "token"));
        let msg = ExecuteMsg::Reset { count: 5 };
        let _res = execute(deps.as_mut(), mock_env(), auth_info, msg).unwrap();

        // should now be 5
        let res = query(deps.as_ref(), mock_env(), QueryMsg::GetCount {}).unwrap();
        let value: GetCountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }*/
}
