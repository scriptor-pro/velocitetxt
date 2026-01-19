use std::fmt;

use rusqlite::params;

use crate::models::NoteMetadata;
use crate::services::{indexer, notes_fs};

#[derive(Debug)]
pub struct SearchError {
  details: String,
}

impl SearchError {
  pub fn new(details: impl Into<String>) -> Self {
    Self {
      details: details.into(),
    }
  }
}

impl fmt::Display for SearchError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{}", self.details)
  }
}

impl std::error::Error for SearchError {}

pub fn search_notes(query: &str) -> Result<Vec<NoteMetadata>, SearchError> {
  if query.trim().is_empty() {
    return notes_fs::list_notes().map_err(|error| SearchError::new(error.to_string()));
  }

  let conn = indexer::open_connection().map_err(|error| SearchError::new(error.to_string()))?;
  indexer::ensure_schema(&conn).map_err(|error| SearchError::new(error.to_string()))?;

  let mut statement = conn
    .prepare(
      "SELECT id, title, layout, date, description, statut, tags, updated, archived
       FROM notes_fts
       WHERE notes_fts MATCH ?
       ORDER BY bm25(notes_fts)",
    )
    .map_err(|error| SearchError::new(format!("Prepare search failed: {}", error)))?;

  let notes = statement
    .query_map(params![query], |row| {
      let tags_raw: String = row.get(6)?;
      let tags = tags_raw
        .split(',')
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
      let archived_raw: String = row.get(8)?;

      Ok(NoteMetadata {
        id: row.get(0)?,
        title: row.get(1)?,
        layout: row.get(2)?,
        date: row.get(3)?,
        description: row.get(4)?,
        statut: row.get(5)?,
        tags,
        updated: row.get(7)?,
        archived: archived_raw == "true",
      })
    })
    .map_err(|error| SearchError::new(format!("Search failed: {}", error)))?;

  let mut results = Vec::new();
  for note in notes {
    results.push(note.map_err(|error| SearchError::new(format!("Row error: {}", error)))?);
  }

  Ok(results)
}
