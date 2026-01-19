use crate::models::{NoteDetail, NoteMetadata};
use crate::services::notes_fs;

#[tauri::command]
pub fn list_notes() -> Result<Vec<NoteMetadata>, String> {
  notes_fs::list_notes().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn get_note(id: String) -> Result<NoteDetail, String> {
  notes_fs::get_note(&id).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_note(title: String) -> Result<NoteDetail, String> {
  notes_fs::create_note(&title).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_note(id: String, content: String) -> Result<NoteDetail, String> {
  notes_fs::save_note(&id, &content).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn archive_note(id: String, archived: bool) -> Result<NoteDetail, String> {
  notes_fs::set_archived(&id, archived).map_err(|error| error.to_string())
}
