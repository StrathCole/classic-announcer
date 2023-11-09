
use std::ops::Add;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    Env, MessageInfo,
    Response, StdResult, Addr, Storage,
};

use cosmwasm_std::DepsMut;

//use schemars::_serde_json::json;
//use serde_json_wasm::from_str;

use crate::error::ContractError;
use crate::migrate::{ensure_from_older_version, set_contract_version};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{WHITELIST, WhitelistAction, WHITELIST_VOTES, WhitelistVote, Announcement, announcements, NEXT_ID};

// version info for migration info
const CONTRACT_NAME: &str = "strathcole:announcer";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let _original_version = ensure_from_older_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default()) 
}

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate (
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    WHITELIST.save(deps.storage, &vec![info.sender])?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::AddToWhitelist { author } => execute_add_to_whitelist(deps, env, info, author),
        ExecuteMsg::RemoveFromWhitelist { author } => execute_remove_from_whitelist(deps, env, info, author),
        ExecuteMsg::Announcement { title, content, topic } => execute_announcement(deps, env, info, title, content, topic),
        ExecuteMsg::DeleteAnnouncement { id } => execute_delete_announcement(deps, env, info, id),
    }
}

fn check_whitelist_confirmation(
    storage: &mut dyn Storage,
    env: Env,
    info: MessageInfo,
    author: Addr,
    action: WhitelistAction,
) -> Result<Response, ContractError> {
    // check if the sender is in the whitelist
    let whitelist = WHITELIST.load(storage)?;
    if !whitelist.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    match action {
        WhitelistAction::Add => {
            // check if the author is already in the whitelist
            if whitelist.contains(&author) {
                return Err(ContractError::GenericError("The author is already in the whitelist".to_string()));
            }
        },
        WhitelistAction::Remove => {
            // check if the author is not in the whitelist
            if !whitelist.contains(&author) {
                return Err(ContractError::GenericError("The author is not in the whitelist".to_string()));
            }
        },
    }

    let new_vote = WhitelistVote {
        action: action.clone(),
        confirmed: vec![],
        expires: env.block.time.plus_seconds(60 * 60 * 24 * 7),
    };

    // check if there is already a voting entry
    let mut votes = if let Ok(votes) = WHITELIST_VOTES.load(storage, author.to_string()) {
        if votes.expires.lt(&env.block.time) || votes.action != action {
            new_vote
        } else {
            votes
        }
    } else {
        new_vote
    };

    // check if the sender has already voted
    // we don't error out here, just ignore the vote
    if !votes.confirmed.contains(&info.sender) {
        // add the sender to the confirmed list
        votes.confirmed.push(info.sender.clone());
    }


    let voters = whitelist.len();
    // check if more than 2/3 of the whitelist has voted
    let required_majority = voters.checked_mul(2)
        .and_then(|v| v.checked_add(2)) // Add 2 for rounding up before division
        .and_then(|v| v.checked_div(3))
        .ok_or(ContractError::GenericError("Cannot calculate majority".to_string()))?;

    if votes.confirmed.len() >= required_majority {
        // update the whitelist
        let mut whitelist = WHITELIST.load(storage)?;
        match votes.action {
            WhitelistAction::Add => {
                whitelist.push(author.clone());
            },
            WhitelistAction::Remove => {
                whitelist.retain(|a| a != &author);
            },
        }
        WHITELIST.save(storage, &whitelist)?;

        // remove the voting entry
        WHITELIST_VOTES.remove(storage, author.to_string());

        return Ok(Response::new()
            .add_attribute("action", votes.action.to_string())
            .add_attribute("author", author.to_string())
            .add_attribute("result", "confirmed"));
    }

    // save the voting entry
    WHITELIST_VOTES.save(storage, author.to_string(), &votes)?;

    Ok(Response::new()
        .add_attribute("action", votes.action.to_string())
        .add_attribute("author", author.to_string())
        .add_attribute("result", "voted"))
}

fn execute_add_to_whitelist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    author: Addr,
) -> Result<Response, ContractError> {
    check_whitelist_confirmation(deps.storage, env, info, author, WhitelistAction::Add)
}

fn execute_remove_from_whitelist(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    author: Addr,
) -> Result<Response, ContractError> {
    check_whitelist_confirmation(deps.storage, env, info, author, WhitelistAction::Remove)
}

fn execute_announcement(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    title: String,
    content: String,
    topic: Option<String>,
) -> Result<Response, ContractError> {
    // check if the sender is in the whitelist
    let whitelist = WHITELIST.load(deps.storage)?;
    if !whitelist.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let next_id = match NEXT_ID.may_load(deps.storage)? {
        Some(id) => id,
        None => 0,
    }.add(1);
    NEXT_ID.save(deps.storage, &next_id)?;

    let announcement = Announcement::new(next_id, title, content, info.sender.clone(), topic, env.block.time);

    announcements().save(deps.storage, next_id, &announcement)?;

    Ok(Response::new()
        .add_attribute("action", "announcement")
        .add_attribute("author", info.sender.to_string())
        .add_attribute("id", next_id.to_string()))
}

fn execute_delete_announcement(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    // check if the sender is in the whitelist
    let whitelist = WHITELIST.load(deps.storage)?;
    if !whitelist.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    announcements().remove(deps.storage, id)?;

    Ok(Response::new()
        .add_attribute("action", "delete_announcement")
        .add_attribute("author", info.sender.to_string())
        .add_attribute("id", id.to_string()))
}