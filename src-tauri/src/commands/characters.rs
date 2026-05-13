use crate::group_chat::memory_graph::{compute_relevance_edges, detect_communities, detect_knowledge_gaps};
use crate::models::{AiCharacter, AllSettings, CommunityInfo, KnowledgeGapInfo, MemoryGraphData, MemoryNode, MemorySource};
use crate::storage;

#[tauri::command]
pub fn list_characters() -> Result<Vec<AiCharacter>, String> {
    log::debug!("[characters] list_characters");
    let settings = storage::settings::get_user_settings();
    Ok(settings.ai_characters)
}

#[tauri::command]
pub fn create_character(
    label: String,
    role_type: String,
    role_instruction: Option<String>,
    default_provider: String,
    default_model: Option<String>,
    icon: Option<String>,
) -> Result<AiCharacter, String> {
    log::debug!("[characters] create_character: label={}", label);
    let trimmed_label = label.trim().to_string();
    if trimmed_label.is_empty() {
        return Err("Character label cannot be empty".to_string());
    }

    let now = crate::models::now_iso();
    let character = AiCharacter {
        id: uuid::Uuid::new_v4().to_string(),
        label: trimmed_label,
        role_type,
        role_instruction,
        default_provider,
        default_model,
        icon,
        avatar_path: None,
        personality: None,
        expertise: vec![],
        memory_config: None,
        created_at: now.clone(),
        updated_at: now,
    };

    let mut all = load_all()?;
    all.user.ai_characters.push(character.clone());
    all.user.updated_at = crate::models::now_iso();
    save_all(&all)?;
    Ok(character)
}

#[tauri::command]
pub fn update_character(
    id: String,
    label: Option<String>,
    role_type: Option<String>,
    role_instruction: Option<Option<String>>,
    default_provider: Option<String>,
    default_model: Option<Option<String>>,
    icon: Option<Option<String>>,
    avatar_path: Option<Option<String>>,
    personality: Option<Option<String>>,
    expertise: Option<Vec<String>>,
    memory_config: Option<Option<crate::models::MemoryConfig>>,
) -> Result<AiCharacter, String> {
    log::debug!("[characters] update_character: id={}", id);
    let mut all = load_all()?;
    let character = all
        .user
        .ai_characters
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or_else(|| format!("Character not found: {}", id))?;

    if let Some(v) = label {
        let trimmed = v.trim().to_string();
        if trimmed.is_empty() {
            return Err("Character label cannot be empty".to_string());
        }
        character.label = trimmed;
    }
    if let Some(v) = role_type {
        character.role_type = v;
    }
    if let Some(v) = role_instruction {
        character.role_instruction = v;
    }
    if let Some(v) = default_provider {
        character.default_provider = v;
    }
    if let Some(v) = default_model {
        character.default_model = v;
    }
    if let Some(v) = icon {
        character.icon = v;
    }
    if let Some(v) = avatar_path {
        character.avatar_path = v;
    }
    if let Some(v) = personality {
        character.personality = v;
    }
    if let Some(v) = expertise {
        character.expertise = v;
    }
    if let Some(v) = memory_config {
        character.memory_config = v;
    }
    character.updated_at = crate::models::now_iso();

    let updated = character.clone();
    all.user.updated_at = crate::models::now_iso();
    save_all(&all)?;
    Ok(updated)
}

#[tauri::command]
pub fn delete_character(id: String) -> Result<(), String> {
    log::debug!("[characters] delete_character: id={}", id);
    let mut all = load_all()?;
    let len_before = all.user.ai_characters.len();
    all.user.ai_characters.retain(|c| c.id != id);
    if all.user.ai_characters.len() == len_before {
        return Err(format!("Character not found: {}", id));
    }
    all.user.updated_at = crate::models::now_iso();
    save_all(&all)
}

// --- Memory CRUD ---

#[tauri::command]
pub async fn list_character_memories(
    character_id: String,
) -> Result<Vec<MemoryNode>, String> {
    storage::characters::read_all_memory_log_entries(&character_id)
}

#[tauri::command]
pub async fn get_character_memory(
    character_id: String,
    memory_id: String,
) -> Result<Option<MemoryNode>, String> {
    let entries = storage::characters::read_all_memory_log_entries(&character_id)?;
    Ok(entries.into_iter().find(|n| n.id == memory_id))
}

#[tauri::command]
pub async fn create_character_memory(
    character_id: String,
    content: String,
    memory_type: String,
    confidence: f64,
    tags: Vec<String>,
) -> Result<MemoryNode, String> {
    let now = chrono::Utc::now().to_rfc3339();
    let node = MemoryNode {
        id: uuid::Uuid::new_v4().to_string(),
        character_id: character_id.clone(),
        content: content.clone(),
        memory_type: memory_type.clone(),
        confidence,
        source: MemorySource {
            kind: "manual".to_string(),
            run_id: None,
            group_chat_id: None,
        },
        tags: tags.clone(),
        created_at: now.clone(),
        updated_at: now,
    };

    // 1. Append to authoritative log
    storage::characters::append_memory_log(&character_id, &node)?;

    // 2. Update graph
    let existing = storage::characters::read_all_memory_log_entries(&character_id)?;
    let mut graph = storage::characters::load_memory_graph(&character_id)?;
    graph.nodes.push(node.clone());
    let new_edges = compute_relevance_edges(&node, &existing, &graph.edges);
    graph.edges.extend(new_edges);
    if let Err(e) = storage::characters::save_memory_graph(&character_id, &graph) {
        log::warn!("[characters] save_memory_graph failed for {}: {}", character_id, e);
    }

    // 3. LanceDB upsert (fire-and-forget if embedding fails)
    if let Ok(embedding_vec) = crate::commands::embedding::fetch_embedding(&content).await {
        let _ = crate::commands::vectorstore::vector_upsert(
            character_id.clone(),
            node.id.clone(),
            embedding_vec,
        )
        .await;
    }

    Ok(node)
}

#[tauri::command]
pub async fn update_character_memory(
    character_id: String,
    memory_id: String,
    content: Option<String>,
    memory_type: Option<String>,
    confidence: Option<f64>,
    tags: Option<Vec<String>>,
) -> Result<MemoryNode, String> {
    let updated = storage::characters::update_memory_in_log(
        &character_id,
        &memory_id,
        content.clone(),
        memory_type,
        confidence,
        tags,
    )?;

    // Update vector if content changed
    if let Some(ref c) = content {
        let _ =
            crate::commands::vectorstore::vector_delete(character_id.clone(), memory_id.clone())
                .await;
        if let Ok(embedding_vec) = crate::commands::embedding::fetch_embedding(c).await {
            let _ = crate::commands::vectorstore::vector_upsert(
                character_id,
                memory_id,
                embedding_vec,
            )
            .await;
        }
    }

    Ok(updated)
}

#[tauri::command]
pub async fn delete_character_memory(
    character_id: String,
    memory_id: String,
) -> Result<(), String> {
    storage::characters::delete_memory_from_log(&character_id, &memory_id)?;
    let _ = crate::commands::vectorstore::vector_delete(character_id, memory_id).await;
    Ok(())
}

#[tauri::command]
pub async fn get_memory_graph(
    character_id: String,
) -> Result<MemoryGraphData, String> {
    storage::characters::load_memory_graph(&character_id)
}

#[tauri::command]
pub async fn get_memory_communities(
    character_id: String,
) -> Result<Vec<CommunityInfo>, String> {
    let graph = storage::characters::load_memory_graph(&character_id)?;
    Ok(detect_communities(&graph.nodes, &graph.edges))
}

#[tauri::command]
pub async fn get_knowledge_gaps(
    character_id: String,
) -> Result<Vec<KnowledgeGapInfo>, String> {
    let graph = storage::characters::load_memory_graph(&character_id)?;
    let communities = detect_communities(&graph.nodes, &graph.edges);
    Ok(detect_knowledge_gaps(&graph.nodes, &graph.edges, &communities))
}

fn load_all() -> Result<AllSettings, String> {
    // Use the same load path as get_user_settings / update_user_settings
    Ok(storage::settings::load())
}

fn save_all(all: &AllSettings) -> Result<(), String> {
    storage::settings::save(all)
}
