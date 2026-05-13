use crate::models::{EmbeddingConfig, TestEmbeddingResult};
use crate::storage::settings;
use serde_json::Value;
use std::time::Instant;
use tauri::command;

#[command]
pub async fn get_embedding_config() -> Result<Option<EmbeddingConfig>, String> {
    Ok(settings::get_embedding_config())
}

#[command]
pub async fn update_embedding_config(config: EmbeddingConfig) -> Result<EmbeddingConfig, String> {
    settings::update_embedding_config(config)
}

#[command]
pub async fn test_embedding_connection() -> Result<TestEmbeddingResult, String> {
    let config = settings::get_embedding_config().ok_or("No embedding config")?;

    if !config.enabled {
        return Err("Embedding is disabled".into());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let start = Instant::now();
    let body = serde_json::json!({
        "input": "test connection",
        "model": config.model,
    });

    let mut req = client.post(&config.endpoint).json(&body);
    if let Some(ref key) = config.api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let latency = start.elapsed().as_millis() as u64;

    if !resp.status().is_success() {
        return Ok(TestEmbeddingResult {
            success: false,
            latency_ms: latency,
            dimension: 0,
            error: Some(format!("HTTP {}", resp.status())),
        });
    }

    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let dimension = json["data"][0]["embedding"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);

    Ok(TestEmbeddingResult {
        success: true,
        latency_ms: latency,
        dimension,
        error: None,
    })
}

/// Fetch embedding vector for a text string from the configured API.
/// Used by memory CRUD and auto-extraction — not a Tauri command.
pub async fn fetch_embedding(text: &str) -> Result<Vec<f32>, String> {
    let config = settings::get_embedding_config().ok_or("No embedding config")?;

    if !config.enabled {
        return Err("Embedding is disabled".into());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;

    let body = serde_json::json!({
        "input": &text[..text.len().min(2000)],
        "model": config.model,
    });

    let mut req = client.post(&config.endpoint).json(&body);
    if let Some(ref key) = config.api_key {
        req = req.header("Authorization", format!("Bearer {}", key));
    }

    let resp = req.send().await.map_err(|e| e.to_string())?;
    let json: Value = resp.json().await.map_err(|e| e.to_string())?;
    let embedding: Vec<f32> = json["data"][0]["embedding"]
        .as_array()
        .ok_or("Missing embedding data")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect();

    Ok(embedding)
}
