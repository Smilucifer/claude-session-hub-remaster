use crate::group_chat::models::{PlanArtifact, PlanStatus, PlanTask, TaskStatus};
use crate::storage;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanTaskInput {
    pub id: Option<String>,
    pub description: String,
    pub assignee_id: Option<String>,
    pub status: Option<String>,
}

fn into_plan_task(input: PlanTaskInput) -> PlanTask {
    let id = input.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let status = input
        .status
        .and_then(|s| serde_json::from_value(serde_json::Value::String(s)).ok())
        .unwrap_or(TaskStatus::Todo);
    PlanTask {
        id,
        description: input.description,
        assignee_id: input.assignee_id,
        status,
    }
}

#[tauri::command]
pub fn get_plan_for_group_chat(
    group_chat_id: String,
) -> Result<Option<PlanArtifact>, String> {
    Ok(storage::group_chats::get_plan_for_group_chat(&group_chat_id))
}

#[tauri::command]
pub fn create_plan(
    group_chat_id: String,
    title: String,
    tasks: Vec<PlanTaskInput>,
) -> Result<PlanArtifact, String> {
    let plan_tasks = tasks.into_iter().map(into_plan_task).collect();
    storage::group_chats::create_plan(&group_chat_id, title, plan_tasks)
}

#[tauri::command]
pub fn update_plan(
    plan_id: String,
    title: Option<String>,
    tasks: Option<Vec<PlanTaskInput>>,
    user_notes: Option<String>,
    clear_user_notes: Option<bool>,
) -> Result<PlanArtifact, String> {
    let plan_tasks = tasks.map(|t| t.into_iter().map(into_plan_task).collect());
    storage::group_chats::update_plan(&plan_id, title, plan_tasks, user_notes, clear_user_notes.unwrap_or(false))
}

#[tauri::command]
pub fn approve_plan(plan_id: String) -> Result<PlanArtifact, String> {
    storage::group_chats::set_plan_status(&plan_id, PlanStatus::Active)
}

#[tauri::command]
pub fn complete_plan(plan_id: String) -> Result<PlanArtifact, String> {
    storage::group_chats::set_plan_status(&plan_id, PlanStatus::Completed)
}
