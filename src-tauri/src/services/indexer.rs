use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::services::notes_fs::{self, NoteIndexEntry};

#[derive(Debug)]
pub struct IndexerError {
  details: String,
}

impl IndexerError {
  pub fn new(details: impl Into<String>) -> Self {
    Self {
      details: details.into(),
    }
  }
}

impl fmt::Display for IndexerError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{}", self.details)
  }
}

impl std::error::Error for IndexerError {}

pub fn rebuild_index() -> Result<(), IndexerError> {
  let conn = open_connection()?;
  ensure_schema(&conn)?;
  conn
    .execute("DELETE FROM notes_fts", [])
    .map_err(|error| IndexerError::new(format!("Clear index failed: {}", error)))?;

  let notes = notes_fs::list_note_entries()
    .map_err(|error| IndexerError::new(format!("Read notes failed: {}", error)))?;
  for entry in notes {
    insert_note(&conn, &entry)?;
  }

  Ok(())
}

pub fn upsert_note_by_path(path: &Path) -> Result<(), IndexerError> {
  let entry = notes_fs::note_entry_from_path(path)
    .map_err(|error| IndexerError::new(format!("Read note failed: {}", error)))?;
  let Some(entry) = entry else {
    return Ok(());
  };

  let conn = open_connection()?;
  ensure_schema(&conn)?;
  delete_note_by_id(&conn, &entry.metadata.id)?;
  insert_note(&conn, &entry)
}

pub fn delete_note_by_path(path: &Path) -> Result<(), IndexerError> {
  let Some(id) = notes_fs::note_id_from_path(path) else {
    return Ok(());
  };

  let conn = open_connection()?;
  ensure_schema(&conn)?;
  delete_note_by_id(&conn, &id)
}

pub fn open_connection() -> Result<Connection, IndexerError> {
  let path = db_path()?;
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|error| IndexerError::new(format!("Create db dir failed: {}", error)))?;
  }

  Connection::open(path).map_err(|error| IndexerError::new(format!("Open db failed: {}", error)))
}

pub fn ensure_schema(conn: &Connection) -> Result<(), IndexerError> {
  conn
    .execute_batch(
      "CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
        id UNINDEXED,
        title,
        layout,
        date,
        description,
        statut,
        tags,
        updated,
        archived,
        body
      );",
    )
    .map_err(|error| IndexerError::new(format!("Create schema failed: {}", error)))
}

fn db_path() -> Result<PathBuf, IndexerError> {
  let home = std::env::var("HOME").map_err(|_| IndexerError::new("HOME not set"))?;
  Ok(PathBuf::from(home)
    .join(".local")
    .join("share")
    .join("velocitext")
    .join("velocitext.db"))
}

fn insert_note(conn: &Connection, entry: &NoteIndexEntry) -> Result<(), IndexerError> {
  let tags = entry.metadata.tags.join(", ");
  let archived = if entry.metadata.archived { "true" } else { "false" };

  conn
    .execute(
      "INSERT INTO notes_fts (id, title, layout, date, description, statut, tags, updated, archived, body)
       VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
      params![
        &entry.metadata.id,
        &entry.metadata.title,
        &entry.metadata.layout,
        &entry.metadata.date,
        &entry.metadata.description,
        &entry.metadata.statut,
        tags,
        &entry.metadata.updated,
        archived,
        &entry.body
      ],
    )
    .map_err(|error| IndexerError::new(format!("Insert note failed: {}", error)))?;

  Ok(())
}

fn delete_note_by_id(conn: &Connection, id: &str) -> Result<(), IndexerError> {
  conn
    .execute("DELETE FROM notes_fts WHERE id = ?1", params![id])
    .map_err(|error| IndexerError::new(format!("Delete note failed: {}", error)))?;
  Ok(())
}
