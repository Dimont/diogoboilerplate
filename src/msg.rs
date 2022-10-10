use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{ Addr, Api, Coin, StdResult };
use cw20::{ Cw20Coin, Cw20ReceiveMsg };

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    List {},
    Details { id: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListResponse {
    pub escrows: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DetailsResponse {
    
    pub id: String,
    /// arbiter can decide to approve or refund the escrow
    pub arbiter: String,
    /// if approved, funds go to the recipient
    pub recipient: Option<String>,
    /// if refunded, funds go to the source
    pub source: String,
    pub title: String,
    pub description: String,
    pub end_height: Option<u64>,
    pub end_time: Option<u64>,
    /// Balance in native tokens
    pub native_balance: Vec<Coin>,
    /// Balance in cw20 tokens
    pub cw20_balance: Vec<Cw20Coin>,
    /// Wish tokens by arbiter
    pub cw20_wishlist: Vec<String>,
    /// Recipient Balance in native tokens
    pub recip_native_balance: Vec<Coin>,
    /// Recipient Balance in cw20 tokens
    pub recip_cw20_balance: Vec<Cw20Coin>,
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateEscrow( CreateMsg ),

    SetRecipient {
        id: String,
        recipient: String,
    },

    Approve {
        id: String,
    },

    Refund {
        id: String,
    },

    /// Adds all sent native tokens to the contract
    TopUp {
        id: String,
    },

    /// Adds all sent native tokens to the contract
    TopUpRecip {
        id: String,
    },

    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CreateMsg {
    //escrow struct
    pub id: String,
    pub arbiter: String,
    pub recipient: Option<String>,
    pub title: String,
    pub description: String,
    pub end_height: Option<u64>,
    pub end_time: Option<u64>,
    pub cw20_wishlist: Option<Vec<String>>,
}

impl CreateMsg {
    pub fn addr_wishlist(&self, api: &dyn Api) -> StdResult<Vec<Addr>> {
        match self.cw20_wishlist.as_ref() {
            Some(v) => v.iter().map(|h| api.addr_validate(h)).collect(),
            None => Ok(vec![]),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    CreateEscrow(CreateMsg),
    /// Adds all sent native tokens to the contract
    TopUp {
        id: String,
    },
    TopUpRecip {
        id: String,
    },
}

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 20 {
        return false;
    }
    true
}