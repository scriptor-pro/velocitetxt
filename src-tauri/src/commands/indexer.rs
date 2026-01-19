use crate::services::indexer;

#[tauri::command]
pub fn rebuild_index() -> Result<(), String> {
  indexer::rebuild_index().map_err(|error| error.to_string())
}
