use cosmwasm_std::{Timestamp, Addr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    AddToWhitelist { authors: Vec<Addr>, },
    RemoveFromWhitelist { authors: Vec<Addr>, },
    Announcement { title: String, content: String, topic: Option<String>, },
    DeleteAnnouncement { id: u64, },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Whitelist {},
    Pending {},
    Announcements(QueryAnnouncementsMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct QueryAnnouncementsMsg {
    pub author: Option<Addr>,
    pub topic: Option<String>,
    pub since: Option<Timestamp>,
}

