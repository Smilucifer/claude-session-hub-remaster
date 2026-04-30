use crate::models::TaskRun;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cwd: Option<String>,
    pub memo: String,
    pub participants: Vec<RoomParticipant>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomParticipant {
    pub id: String,
    pub run_id: String,
    pub agent: String,
    pub label: String,
    pub role: String,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoomSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cwd: Option<String>,
    pub participant_count: usize,
    pub memo_preview: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomParticipantDetail {
    pub participant: RoomParticipant,
    pub run: Option<TaskRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomDetail {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cwd: Option<String>,
    pub memo: String,
    pub participants: Vec<RoomParticipantDetail>,
    pub created_at: String,
    pub updated_at: String,
}
