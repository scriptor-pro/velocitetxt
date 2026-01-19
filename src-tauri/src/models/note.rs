use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoteMetadata {
  pub id: String,
  pub title: String,
  pub layout: String,
  pub date: String,
  pub description: String,
  pub statut: String,
  pub tags: Vec<String>,
  pub updated: String,
  pub archived: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NoteDetail {
  pub metadata: NoteMetadata,
  pub content: String,
}
