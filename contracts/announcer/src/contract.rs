
use std::ops::Add;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{
    Env, MessageInfo,
    Response, StdResult, Addr, Order,
};

use cosmwasm_std::DepsMut;

//use schemars::_serde_json::json;
//use serde_json_wasm::from_str;

use crate::error::ContractError;
use crate::migrate::{ensure_from_older_version, set_contract_version};
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg};
use crate::state::{WHITELIST, WhitelistAction, WHITELIST_VOTES, WhitelistVote, Announcement, announcements, NEXT_ID, TOPICS, Topic};

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
        ExecuteMsg::AddToWhitelist { authors } => check_whitelist_confirmation(deps, env, info, authors, WhitelistAction::Add),
        ExecuteMsg::RemoveFromWhitelist { authors } => check_whitelist_confirmation(deps, env, info, authors, WhitelistAction::Remove),
        ExecuteMsg::Announcement { title, content, topic } => execute_announcement(deps, env, info, title, content, topic),
        ExecuteMsg::DeleteAnnouncement { id } => execute_delete_announcement(deps, env, info, id),
        ExecuteMsg::AddTopic { identifier, name, description, color } => execute_add_topic(deps, env, info, identifier, name, description, color),
        ExecuteMsg::RemoveTopic { identifier } => execute_remove_topic(deps, env, info, identifier),
    }
}

fn check_whitelist_confirmation(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    authors: Vec<Addr>,
    action: WhitelistAction,
) -> Result<Response, ContractError> {
    // check if the sender is in the whitelist
    let whitelist = WHITELIST.load(deps.storage)?;
    if !whitelist.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    if authors.len() == 0 {
        return Err(ContractError::GenericError("No authors provided".to_string()));
    }

    // first of all remove expired votes
    let expired = match WHITELIST_VOTES.range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| match item {
            Ok((k, a)) if a.expires > env.block.time => Some(Ok(k)),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
        .collect::<StdResult<Vec<String>>>() {
            Ok(expired) => expired,
            Err(_) => vec![],
    };

    for vote_key in expired {
        WHITELIST_VOTES.remove(deps.storage, vote_key);
    }

    // now process the new votes
    let mut processed = 0;
    let mut voted = 0;
    let mut confirmed = 0;

    for author in authors.clone() {    
        match action {
            WhitelistAction::Add => {
                // check if the author is already in the whitelist
                if !whitelist.contains(&author) {
                    processed += 1;
                }
            },
            WhitelistAction::Remove => {
                // check if the author is not in the whitelist
                if whitelist.contains(&author) {
                    processed += 1;
                }
            },
        }

        let new_vote = WhitelistVote {
            action: action.clone(),
            confirmed: vec![],
            expires: env.block.time.plus_seconds(60 * 60 * 24 * 7),
        };

        // check if there is already a voting entry
        let mut votes = if let Ok(votes) = WHITELIST_VOTES.load(deps.storage, author.to_string()) {
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
            let mut whitelist = WHITELIST.load(deps.storage)?;
            match votes.action {
                WhitelistAction::Add => {
                    whitelist.push(author.clone());
                },
                WhitelistAction::Remove => {
                    whitelist.retain(|a| a != &author);
                },
            }
            WHITELIST.save(deps.storage, &whitelist)?;

            // remove the voting entry
            WHITELIST_VOTES.remove(deps.storage, author.to_string());
            confirmed += 1;
        } else {
            WHITELIST_VOTES.save(deps.storage, author.to_string(), &votes)?;
            voted += 1;
        }
    }

    Ok(Response::new()
    .add_attribute("action", action.to_string())
    .add_attribute("author", authors.iter().map(|a| a.to_string()).collect::<Vec<String>>().join(",").to_string())
    .add_attribute("processed", processed.to_string())
    .add_attribute("confirmed", confirmed.to_string())
    .add_attribute("pending", voted.to_string())
    )
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

    let topic = if let Some(topic) = topic {
        match TOPICS.load(deps.storage, topic) {
            Ok(topic) => {
                Some(topic.clone())
            },
            Err(_) => { return Err(ContractError::GenericError("Topic not found".to_string())); },
        }
    } else {
        None
    };

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

fn execute_add_topic(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    identifier: String,
    name: String,
    description: String,
    color: String,
) -> Result<Response, ContractError> {
    // check if the sender is in the whitelist
    let whitelist = WHITELIST.load(deps.storage)?;
    if !whitelist.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let topic = Topic::new(identifier.clone(), name, description, color);
    TOPICS.update(deps.storage, identifier.clone(), |old| {
        match old {
            Some(_) => Err(ContractError::GenericError("Topic already exists".to_string())),
            None => Ok(topic),
        }
    })?;

    Ok(Response::new()
        .add_attribute("action", "add_topic")
        .add_attribute("author", info.sender.to_string())
        .add_attribute("identifier", identifier))
}

fn execute_remove_topic(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    identifier: String,
) -> Result<Response, ContractError> {
    // check if the sender is in the whitelist
    let whitelist = WHITELIST.load(deps.storage)?;
    if !whitelist.contains(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    // check if there are any announcements with this topic
    let list = announcements().idx.topic.sub_prefix(identifier.clone()).range(deps.storage, None, None, Order::Descending)
        .filter_map(Result::ok).map(|(_, a)| a).collect::<Vec<Announcement>>();
    if list.len() > 0 {
        return Err(ContractError::GenericError("Topic still in use by at least one announcement".to_string()));
    }

    TOPICS.remove(deps.storage, identifier.clone());

    Ok(Response::new()
        .add_attribute("action", "remove_topic")
        .add_attribute("author", info.sender.to_string())
        .add_attribute("identifier", identifier))
}