use core::fmt;
use std::fmt::{Display, Formatter};

use cosmwasm_std::{Addr, Timestamp};

use cw_storage_plus::{IndexedMap, IndexList, Item, MultiIndex, Index, Map};

use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

//use cw_utils::Duration;

pub const WHITELIST: Item<Vec<Addr>> = Item::new("whitelist");
pub const TOPICS: Map<String, Topic> = Map::new("topics");

pub const NEXT_ID: Item<u64> = Item::new("next_id");

pub const WHITELIST_VOTES: Map<String, WhitelistVote> = Map::new("whitelist_votes");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum WhitelistAction {
    Add,
    Remove,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct WhitelistVote {
    pub confirmed: Vec<Addr>,
    pub action: WhitelistAction,
    pub expires: Timestamp,
}

impl Display for WhitelistAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            WhitelistAction::Add => write!(f, "add"),
            WhitelistAction::Remove => write!(f, "remove"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Topic {
    pub identifier: String,
    pub name: String,
    pub description: String,
    pub color: String,
}

impl Topic {
    pub fn new(identifier: String, name: String, description: String, color: String) -> Self {
        Self {
            identifier,
            name,
            description,
            color,
        }
    }
}

impl Default for Topic {
    fn default() -> Self {
        Self::new("".to_string(), "".to_string(), "".to_string(), "".to_string())
    }
}

impl Display for Topic {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Announcement {
    pub id: u64,
    pub title: String,
    pub content: String,
    pub author: Addr,
    pub topic: Option<Topic>,
    pub time: Timestamp,
}

impl Announcement {
    pub fn new(id: u64, title: String, content: String, author: Addr, topic: Option<Topic>, time: Timestamp) -> Self {
        Self {
            id,
            title,
            content,
            author,
            topic,
            time,
        }
    }
}

impl Default for Announcement {
    fn default() -> Self {
        Self::new(0, "".to_string(), "".to_string(), Addr::unchecked("".to_string()), None, Timestamp::from_seconds(0))
    }
}

pub struct AnnouncementIndexes<'a> {
    pub author: MultiIndex<'a, (Addr, u64), Announcement, u64>,
    pub time: MultiIndex<'a, u64, Announcement, u64>,
    pub topic: MultiIndex<'a, (String, u64), Announcement, u64>,
}

pub fn announcements<'a>() -> IndexedMap<'a, u64, Announcement, AnnouncementIndexes<'a>> {

    let indexes = AnnouncementIndexes {
        author: MultiIndex::new(|o| (o.author.clone(), o.time.seconds()), "announcements", "an_author"),
        time: MultiIndex::new(|o| o.time.seconds(), "announcements", "an_time"),
        topic: MultiIndex::new(|o| (o.topic.clone().unwrap_or_default().to_string(), o.time.seconds()), "announcements", "an_topic"),
    };

    IndexedMap::new("announcements", indexes)
}

impl<'a> IndexList<Announcement> for AnnouncementIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Announcement>> + '_> {
        let v: Vec<&dyn Index<Announcement>> = vec![&self.author, &self.time, &self.topic];
        Box::new(v.into_iter())
    }
}
