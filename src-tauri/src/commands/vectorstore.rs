use crate::models::VectorSearchResult;
use crate::storage::characters;
use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema};
use futures_util::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::connect;
use std::sync::Arc;
use tauri::command;

const TABLE_NAME: &str = "character_memories";

fn lancedb_path(character_id: &str) -> String {
    characters::char_dir(character_id)
        .join("lancedb")
        .to_string_lossy()
        .to_string()
}

#[command]
pub async fn vector_upsert(
    character_id: String,
    page_id: String,
    vector: Vec<f32>,
) -> Result<(), String> {
    let db_path = lancedb_path(&character_id);
    std::fs::create_dir_all(&db_path).map_err(|e| e.to_string())?;
    let db = connect(&db_path).execute().await.map_err(|e| e.to_string())?;

    let dim = vector.len() as i32;
    if dim == 0 {
        return Err("Empty vector".into());
    }
    let schema = Arc::new(Schema::new(vec![
        Field::new("page_id", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dim,
            ),
            false,
        ),
    ]));

    let table_names = db
        .table_names()
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let table = if table_names.contains(&TABLE_NAME.to_string()) {
        db.open_table(TABLE_NAME)
            .execute()
            .await
            .map_err(|e| e.to_string())?
    } else {
        db.create_empty_table(TABLE_NAME, schema.clone())
            .execute()
            .await
            .map_err(|e| e.to_string())?
    };

    // Delete existing entry for this page_id
    let escaped = page_id.replace('\'', "''");
    let filter = format!("page_id = '{}'", escaped);
    if let Err(e) = table.delete(&filter).await {
        log::warn!("vector_upsert: failed to delete old entry for {}: {}", page_id, e);
    }

    // Build RecordBatch with one row
    let page_ids = StringArray::from(vec![page_id.as_str()]);
    let values = Float32Array::from(vector);
    let list_array = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        dim,
        Arc::new(values),
        None,
    );

    let batch =
        RecordBatch::try_new(schema, vec![Arc::new(page_ids), Arc::new(list_array)])
            .map_err(|e| e.to_string())?;

    table.add(batch).execute().await.map_err(|e| e.to_string())?;
    Ok(())
}

#[command]
pub async fn vector_search(
    character_id: String,
    query_vector: Vec<f32>,
    top_k: u32,
) -> Result<Vec<VectorSearchResult>, String> {
    let db_path = lancedb_path(&character_id);
    let db = connect(&db_path).execute().await.map_err(|e| e.to_string())?;

    let table_names = db
        .table_names()
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    if !table_names.contains(&TABLE_NAME.to_string()) {
        return Ok(Vec::new());
    }

    let table = db
        .open_table(TABLE_NAME)
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let stream = table
        .vector_search(query_vector)
        .map_err(|e| e.to_string())?
        .limit(top_k as usize)
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let batches: Vec<RecordBatch> = stream
        .try_collect()
        .await
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for batch in batches {
        for i in 0..batch.num_rows() {
            let page_id = batch
                .column_by_name("page_id")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .map(|a| a.value(i).to_string())
                .unwrap_or_default();
            let distance = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>())
                .map(|a| a.value(i))
                .unwrap_or(0.0);
            let score = 1.0 / (1.0 + distance as f64);
            out.push(VectorSearchResult {
                page_id,
                score,
                memory: None,
            });
        }
    }
    Ok(out)
}

#[command]
pub async fn vector_delete(
    character_id: String,
    page_id: String,
) -> Result<(), String> {
    let db_path = lancedb_path(&character_id);
    let db = connect(&db_path).execute().await.map_err(|e| e.to_string())?;

    let table_names = db
        .table_names()
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    if !table_names.contains(&TABLE_NAME.to_string()) {
        return Ok(());
    }

    let table = db
        .open_table(TABLE_NAME)
        .execute()
        .await
        .map_err(|e| e.to_string())?;

    let escaped = page_id.replace('\'', "''");
    let filter = format!("page_id = '{}'", escaped);
    if let Err(e) = table.delete(&filter).await {
        log::warn!("vector_delete: failed to delete entry for {}: {}", page_id, e);
    }
    Ok(())
}

#[command]
pub async fn reset_vector_store(
    character_id: String,
) -> Result<usize, String> {
    let db_path = lancedb_path(&character_id);
    let _ = std::fs::remove_dir_all(&db_path);

    // Count non-empty lines in the memory log file (cheaper than deserializing)
    let log_path = characters::memory_log_path(&character_id);
    let count = std::fs::read_to_string(&log_path)
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);

    Ok(count)
}
