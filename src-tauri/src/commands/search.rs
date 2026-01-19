use crate::models::NoteMetadata;
use crate::services::search;

#[tauri::command]
pub fn search_notes(query: String) -> Result<Vec<NoteMetadata>, String> {
  search::search_notes(&query).map_err(|error| error.to_string())
}
