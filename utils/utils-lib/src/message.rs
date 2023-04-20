use std::{collections::BTreeMap, str::FromStr};

use crate::nats::JetStreamable;
use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventCategory {
    Volume,
    Nexus,
    Pool, 
    Replica,
}

impl FromStr for EventCategory {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "volume" => Ok(EventCategory::Volume),
            "nexus" => Ok(EventCategory::Nexus),
            "pool" => Ok(EventCategory::Pool),
            "replica" => Ok(EventCategory::Replica),
            _ => Err(anyhow!(
                "The string {:?} does not describe a valid category.",
                s
            )),
        }
    }
}

impl ToString for EventCategory {
    fn to_string(&self) -> String {
        match self {
            EventCategory::Volume => "volume".to_string(),
            EventCategory::Nexus => "nexus".to_string(),
            EventCategory::Pool => "pool".to_string(),
            EventCategory::Replica => "replica".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventAction {
    Created,
    Deleted,
}

impl FromStr for EventAction {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "created" => Ok(EventAction::Created),
            "deleted" => Ok(EventAction::Deleted),
            _ => Err(anyhow!(
                "The string {:?} does not describe a valid category.",
                s
            )),
        }
    }
}

impl ToString for EventAction {
    fn to_string(&self) -> String {
        match self {
            EventAction::Created => "created".to_string(),
            EventAction::Deleted => "deleted".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EventMessage {
    id: String,
    pub category: String,
    pub action: String,
    pub target: String,
    pub node: String,
}

impl JetStreamable for EventMessage {
    fn subject(&self) -> String {
        format!("stats.events.{}", self.category)
    }
}

impl EventMessage {
    pub fn from_event(event: BTreeMap<String, String>) -> Option<EventMessage> {
        match event.get("event").unwrap().as_str() {
            "VolumeCreated" => Some(EventMessage {
                id: Self::new_random(),
                category: EventCategory::Volume.to_string(),
                action: EventAction::Created.to_string(),
                target: event.get("target").unwrap().to_string(),
                node: event.get("node").unwrap().to_string(),
            }),
            "VolumeDeleted" => Some(EventMessage {
                id: Self::new_random(),
                category: EventCategory::Volume.to_string(),
                action: EventAction::Deleted.to_string(),
                target: event.get("target").unwrap().to_string(),
                node: event.get("node").unwrap().to_string(),
            }),
            "NexusCreated" => Some(EventMessage {
                id: Self::new_random(),
                category: EventCategory::Nexus.to_string(),
                action: EventAction::Created.to_string(),
                target: event.get("target").unwrap().to_string(),
                node: event.get("node").unwrap().to_string(),
            }),
            "NexusDeleted" => Some(EventMessage {
                id: Self::new_random(),
                category: EventCategory::Nexus.to_string(),
                action: EventAction::Deleted.to_string(),
                target: event.get("target").unwrap().to_string(),
                node: event.get("node").unwrap().to_string(),
            }),
            "PoolCreated" => Some(EventMessage {
                id: Self::new_random(),
                category: EventCategory::Pool.to_string(),
                action: EventAction::Created.to_string(),
                target: event.get("target").unwrap().to_string(),
                node: event.get("node").unwrap().to_string(),
            }),
            "PoolDeleted" => Some(EventMessage {
                id: Self::new_random(),
                category: EventCategory::Pool.to_string(),
                action: EventAction::Deleted.to_string(),
                target: event.get("target").unwrap().to_string(),
                node: event.get("node").unwrap().to_string(),
            }),
            "ReplicaCreated" => Some(EventMessage {
                id: Self::new_random(),
                category: EventCategory::Replica.to_string(),
                action: EventAction::Created.to_string(),
                target: event.get("target").unwrap().to_string(),
                node: event.get("node").unwrap().to_string(),
            }),
            "ReplicaDeleted" => Some(EventMessage {
                id: Self::new_random(),
                category: EventCategory::Replica.to_string(),
                action: EventAction::Deleted.to_string(),
                target: event.get("target").unwrap().to_string(),
                node: event.get("node").unwrap().to_string(),
            }),
            _ => {
                println!("Unexpected event message");
                None
            }
        }
    }

    pub fn new_random() -> String {
        let id = Uuid::new_v4();
        id.to_string()
    }
}
