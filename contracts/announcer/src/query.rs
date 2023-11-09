#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

use cosmwasm_std::{Env, StdResult, Binary, to_binary, Order, Addr, Deps};
use cw_storage_plus::Bound;

use crate::{msg::{QueryMsg, QueryAnnouncementsMsg}, state::{WHITELIST, announcements, Announcement, WhitelistVote, WHITELIST_VOTES}};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Announcements(msg) => to_binary(&query_announcements(deps, env, msg)?),
        QueryMsg::Whitelist {  } => to_binary(&query_whitelist(deps, env)?),
        QueryMsg::Pending {  } => to_binary(&query_pending(deps, env)?),
    }
}

fn query_whitelist(deps: Deps, _env: Env) -> StdResult<Vec<Addr>> {
    let whitelist = WHITELIST.load(deps.storage)?;
    Ok(whitelist)
}

fn query_announcements(deps: Deps, _env: Env, msg: QueryAnnouncementsMsg) -> StdResult<Vec<Announcement>> {
    
    let start_at = match msg.since {
        Some(since) => Some(Bound::inclusive((since.seconds(), 0u64))),
        None => None,
    };

    let list = if let Some(author) = msg.author {
        announcements().idx.author.sub_prefix(author).range(deps.storage, start_at, None, Order::Descending)
    } else if let Some(topic) = msg.topic {
        announcements().idx.topic.sub_prefix(topic).range(deps.storage, start_at, None, Order::Descending)
    } else {
        announcements().idx.time.range(deps.storage, start_at, None, Order::Descending)
    }.filter_map(Result::ok).map(|(_, a)| a).collect::<Vec<Announcement>>();
    
    Ok(list)
}

fn query_pending(deps: Deps, env: Env) -> StdResult<Vec<WhitelistVote>> {
    let list = WHITELIST_VOTES.range(deps.storage, None, None, Order::Ascending)
        .filter_map(|item| match item {
            Ok((_, a)) if a.expires > env.block.time => Some(Ok(a)),
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
        .collect::<StdResult<Vec<WhitelistVote>>>()?;
    Ok(list)
}
