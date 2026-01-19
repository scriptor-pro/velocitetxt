use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate};
use regex::Regex;
use serde::Deserialize;

use crate::models::{NoteDetail, NoteMetadata};

const MAX_SLUG_LEN: usize = 101;
const FORBIDDEN_CHARS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
const DEFAULT_LAYOUT: &str = "note.njk";
const DEFAULT_STATUT: &str = "idee";
const STATUTS: [&str; 3] = ["idee", "chantier", "termine"];

#[derive(Debug)]
pub struct NotesError {
  details: String,
}

impl NotesError {
  pub fn new(details: impl Into<String>) -> Self {
    Self {
      details: details.into(),
    }
  }
}

impl fmt::Display for NotesError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{}", self.details)
  }
}

impl std::error::Error for NotesError {}

#[derive(Debug, Deserialize, Clone, PartialEq)]
struct Frontmatter {
  title: String,
  layout: String,
  date: String,
  #[serde(default)]
  description: String,
  statut: String,
  #[serde(default)]
  tags: Vec<String>,
  updated: String,
  archived: bool,
}

pub struct NoteIndexEntry {
  pub metadata: NoteMetadata,
  pub body: String,
}

struct ParsedNote {
  frontmatter: Frontmatter,
  body: String,
  raw: String,
}

pub fn note_id_from_path(path: &Path) -> Option<String> {
  if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
    return None;
  }

  path
    .file_name()
    .and_then(|name| name.to_str())
    .map(str::to_string)
}

pub fn note_entry_from_path(path: &Path) -> Result<Option<NoteIndexEntry>, NotesError> {
  if !path.exists() {
    return Ok(None);
  }

  let file_name = match note_id_from_path(path) {
    Some(name) => name,
    None => return Ok(None),
  };

  let parsed = read_note_file(path)?;
  Ok(Some(NoteIndexEntry {
    metadata: metadata_from_frontmatter(file_name, &parsed.frontmatter),
    body: parsed.body,
  }))
}

pub fn list_notes() -> Result<Vec<NoteMetadata>, NotesError> {
  let notes_dir = notes_dir()?;
  if !notes_dir.exists() {
    return Ok(Vec::new());
  }

  let mut notes = Vec::new();
  let entries = fs::read_dir(&notes_dir)
    .map_err(|error| NotesError::new(format!("Cannot read notes dir: {}", error)))?;

  for entry in entries {
    let entry = entry.map_err(|error| NotesError::new(format!("Read entry failed: {}", error)))?;
    let path = entry.path();
    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
      continue;
    }

    let parsed = read_note_file(&path)?;
    let file_name = file_name(&path)?;
    notes.push(metadata_from_frontmatter(file_name, &parsed.frontmatter));
  }

  notes.sort_by(|left, right| {
    let left_date = parse_date(&left.date);
    let right_date = parse_date(&right.date);
    right_date
      .cmp(&left_date)
      .then_with(|| right.title.cmp(&left.title))
  });
  Ok(notes)
}

pub fn list_note_entries() -> Result<Vec<NoteIndexEntry>, NotesError> {
  let notes_dir = notes_dir()?;
  if !notes_dir.exists() {
    return Ok(Vec::new());
  }

  let mut notes = Vec::new();
  let entries = fs::read_dir(&notes_dir)
    .map_err(|error| NotesError::new(format!("Cannot read notes dir: {}", error)))?;

  for entry in entries {
    let entry = entry.map_err(|error| NotesError::new(format!("Read entry failed: {}", error)))?;
    let path = entry.path();
    if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
      continue;
    }

    let parsed = read_note_file(&path)?;
    let file_name = file_name(&path)?;
    notes.push(NoteIndexEntry {
      metadata: metadata_from_frontmatter(file_name, &parsed.frontmatter),
      body: parsed.body,
    });
  }

  Ok(notes)
}

pub fn get_note(id: &str) -> Result<NoteDetail, NotesError> {
  let notes_dir = notes_dir()?;
  let path = safe_note_path(&notes_dir, id)?;
  if !path.exists() {
    return Err(NotesError::new("Note not found"));
  }

  let parsed = read_note_file(&path)?;
  Ok(NoteDetail {
    metadata: metadata_from_frontmatter(file_name(&path)?, &parsed.frontmatter),
    content: parsed.raw,
  })
}

pub fn create_note(title: &str) -> Result<NoteDetail, NotesError> {
  let notes_dir = ensure_notes_dir()?;
  let now = Local::now();
  let date = now.format("%d-%m-%Y").to_string();
  let time = now.format("%H-%M").to_string();
  let updated = now.format("%d-%m-%Y %H:%M").to_string();

  let slug = slugify_title(title);
  let prefix = format!("{}_{}--", date, time);
  let base_name = build_filename(&prefix, &slug);
  let file_name = ensure_unique_filename(&notes_dir, &base_name, None);
  let frontmatter = Frontmatter {
    title: title.to_string(),
    layout: DEFAULT_LAYOUT.to_string(),
    date,
    description: String::new(),
    statut: DEFAULT_STATUT.to_string(),
    tags: Vec::new(),
    updated,
    archived: false,
  };

  let content = build_content(&frontmatter, "");
  let path = notes_dir.join(&file_name);
  fs::write(&path, &content)
    .map_err(|error| NotesError::new(format!("Write file failed: {}", error)))?;

  Ok(NoteDetail {
    metadata: metadata_from_frontmatter(file_name, &frontmatter),
    content,
  })
}

pub fn save_note(id: &str, content: &str) -> Result<NoteDetail, NotesError> {
  let notes_dir = ensure_notes_dir()?;
  let path = safe_note_path(&notes_dir, id)?;
  if !path.exists() {
    return Err(NotesError::new("Note not found"));
  }

  let existing = read_note_file(&path)?;
  let (mut frontmatter, body) = parse_frontmatter(content)?;

  let mut existing_compare = existing.frontmatter.clone();
  existing_compare.updated.clear();
  let mut incoming_compare = frontmatter.clone();
  incoming_compare.updated.clear();

  let content_changed = existing.body != body || existing_compare != incoming_compare;
  if content_changed && frontmatter.updated == existing.frontmatter.updated {
    frontmatter.updated = Local::now().format("%d-%m-%Y %H:%M").to_string();
  }

  let prefix = extract_prefix(id).unwrap_or_else(current_prefix);
  let slug = slugify_title(&frontmatter.title);
  let target_name = if frontmatter.title != existing.frontmatter.title {
    let base = build_filename(&prefix, &slug);
    ensure_unique_filename(&notes_dir, &base, Some(id))
  } else {
    id.to_string()
  };

  let normalized_content = build_content(&frontmatter, &body);
  let target_path = notes_dir.join(&target_name);
  fs::write(&target_path, &normalized_content)
    .map_err(|error| NotesError::new(format!("Write file failed: {}", error)))?;

  if target_path != path {
    fs::remove_file(&path)
      .map_err(|error| NotesError::new(format!("Remove old file failed: {}", error)))?;
  }

  Ok(NoteDetail {
    metadata: metadata_from_frontmatter(target_name, &frontmatter),
    content: normalized_content,
  })
}

pub fn set_archived(id: &str, archived: bool) -> Result<NoteDetail, NotesError> {
  let notes_dir = ensure_notes_dir()?;
  let path = safe_note_path(&notes_dir, id)?;
  if !path.exists() {
    return Err(NotesError::new("Note not found"));
  }

  let existing = read_note_file(&path)?;
  let mut frontmatter = existing.frontmatter.clone();
  if frontmatter.archived == archived {
    return Ok(NoteDetail {
      metadata: metadata_from_frontmatter(id.to_string(), &frontmatter),
      content: existing.raw,
    });
  }

  frontmatter.archived = archived;
  if frontmatter.updated == existing.frontmatter.updated {
    frontmatter.updated = Local::now().format("%d-%m-%Y %H:%M").to_string();
  }

  let normalized_content = build_content(&frontmatter, &existing.body);
  fs::write(&path, &normalized_content)
    .map_err(|error| NotesError::new(format!("Write file failed: {}", error)))?;

  Ok(NoteDetail {
    metadata: metadata_from_frontmatter(id.to_string(), &frontmatter),
    content: normalized_content,
  })
}

pub fn slugify_title(title: &str) -> String {
  let mut slug = String::new();

  for ch in title.chars() {
    if ch == ' ' {
      slug.push('_');
    } else if FORBIDDEN_CHARS.contains(&ch) {
      slug.push('-');
    } else {
      slug.push(ch);
    }

    if slug.chars().count() >= MAX_SLUG_LEN {
      break;
    }
  }

  if slug.is_empty() {
    "note".to_string()
  } else {
    slug
  }
}

fn validate_note_id(id: &str) -> Result<(), NotesError> {
  // Ensure the ID is a valid filename without path separators
  if id.contains('/') || id.contains('\\') {
    return Err(NotesError::new("Invalid note ID: contains path separators"));
  }

  // Ensure the ID doesn't contain relative path components
  if id.contains("..") {
    return Err(NotesError::new("Invalid note ID: contains relative path"));
  }

  // Ensure the ID ends with .md
  if !id.ends_with(".md") {
    return Err(NotesError::new("Invalid note ID: must be a markdown file"));
  }

  // Validate against expected pattern: DD-MM-YYYY_HH-MM--slug.md
  let note_pattern = Regex::new(r"^\d{2}-\d{2}-\d{4}_\d{2}-\d{2}--[a-zA-Z0-9_\-]+(?:_\d+)?\.md$")
    .map_err(|_| NotesError::new("Regex compilation failed"))?;
  
  if !note_pattern.is_match(id) {
    return Err(NotesError::new("Invalid note ID format"));
  }

  Ok(())
}

fn safe_note_path(notes_dir: &Path, id: &str) -> Result<PathBuf, NotesError> {
  validate_note_id(id)?;
  
  let path = notes_dir.join(id);
  
  // Ensure the resolved path is still within the notes directory
  let canonical_notes_dir = notes_dir.canonicalize()
    .map_err(|_| NotesError::new("Cannot resolve notes directory"))?;
  
  let canonical_path = path.canonicalize()
    .unwrap_or_else(|_| {
      // If file doesn't exist yet, check the parent directory
      if let Some(parent) = path.parent() {
        parent.canonicalize().unwrap_or(path.clone())
      } else {
        path.clone()
      }
    });

  if !canonical_path.starts_with(&canonical_notes_dir) {
    return Err(NotesError::new("Note path outside allowed directory"));
  }

  Ok(path)
}

pub fn notes_dir() -> Result<PathBuf, NotesError> {
  let home = std::env::var("HOME").map_err(|_| NotesError::new("HOME not set"))?;
  Ok(PathBuf::from(home).join("Notes").join("Velocitext"))
}

fn ensure_notes_dir() -> Result<PathBuf, NotesError> {
  let dir = notes_dir()?;
  if !dir.exists() {
    fs::create_dir_all(&dir)
      .map_err(|error| NotesError::new(format!("Create notes dir failed: {}", error)))?;
  }
  Ok(dir)
}

fn read_note_file(path: &Path) -> Result<ParsedNote, NotesError> {
  let raw = fs::read_to_string(path)
    .map_err(|error| NotesError::new(format!("Read file failed: {}", error)))?;
  let (frontmatter, body) = parse_frontmatter(&raw)?;
  Ok(ParsedNote {
    frontmatter,
    body,
    raw,
  })
}

fn parse_frontmatter(content: &str) -> Result<(Frontmatter, String), NotesError> {
  let mut lines = content.lines();
  let first_line = lines
    .next()
    .ok_or_else(|| NotesError::new("Empty file"))?;

  if first_line.trim() != "---" {
    return Err(NotesError::new("Missing frontmatter"));
  }

  let mut yaml_lines = Vec::new();
  let mut found_end = false;

  for line in &mut lines {
    if line.trim() == "---" {
      found_end = true;
      break;
    }
    yaml_lines.push(line);
  }

  if !found_end {
    return Err(NotesError::new("Frontmatter not closed"));
  }

  let yaml = yaml_lines.join("\n");
  let body = lines.collect::<Vec<_>>().join("\n");

  let frontmatter: Frontmatter = serde_yaml::from_str(&yaml)
    .map_err(|error| NotesError::new(format!("Invalid frontmatter: {}", error)))?;

  validate_frontmatter(&frontmatter)?;
  Ok((frontmatter, body))
}

fn validate_frontmatter(frontmatter: &Frontmatter) -> Result<(), NotesError> {
  if frontmatter.layout != DEFAULT_LAYOUT {
    return Err(NotesError::new("Invalid layout"));
  }

  if !STATUTS.contains(&frontmatter.statut.as_str()) {
    return Err(NotesError::new("Invalid statut"));
  }

  if frontmatter.title.trim().is_empty() {
    return Err(NotesError::new("Missing title"));
  }

  if frontmatter.date.trim().is_empty() || frontmatter.updated.trim().is_empty() {
    return Err(NotesError::new("Missing date fields"));
  }

  Ok(())
}

fn metadata_from_frontmatter(id: String, frontmatter: &Frontmatter) -> NoteMetadata {
  NoteMetadata {
    id,
    title: frontmatter.title.clone(),
    layout: frontmatter.layout.clone(),
    date: frontmatter.date.clone(),
    description: frontmatter.description.clone(),
    statut: frontmatter.statut.clone(),
    tags: frontmatter.tags.clone(),
    updated: frontmatter.updated.clone(),
    archived: frontmatter.archived,
  }
}

fn build_content(frontmatter: &Frontmatter, body: &str) -> String {
  let header = format_frontmatter(frontmatter);
  if body.is_empty() {
    header
  } else {
    format!("{}\n{}", header, body)
  }
}

fn format_frontmatter(frontmatter: &Frontmatter) -> String {
  let title = escape_yaml_string(&frontmatter.title);
  let description = escape_yaml_string(&frontmatter.description);
  let tags = if frontmatter.tags.is_empty() {
    "tags: []\n".to_string()
  } else {
    let tag_lines = frontmatter
      .tags
      .iter()
      .map(|tag| format!("  - {}", escape_yaml_string(tag)))
      .collect::<Vec<_>>()
      .join("\n");
    format!("tags:\n{}\n", tag_lines)
  };

  format!(
    "---\n
title: \"{}\"\nlayout: {}\ndate: \"{}\"\ndescription: \"{}\"\nstatut: {}\n{}updated: \"{}\"\narchived: {}\n---\n",
    title,
    frontmatter.layout,
    frontmatter.date,
    description,
    frontmatter.statut,
    tags,
    frontmatter.updated,
    frontmatter.archived
  )
}

fn escape_yaml_string(value: &str) -> String {
  value.replace('"', "\\\"")
}

fn build_filename(prefix: &str, slug: &str) -> String {
  let safe_slug = if slug.is_empty() { "note" } else { slug };
  format!("{}{}.md", prefix, safe_slug)
}

fn ensure_unique_filename(dir: &Path, base: &str, current: Option<&str>) -> String {
  if let Some(id) = current {
    if id == base {
      return base.to_string();
    }
  }

  if !dir.join(base).exists() {
    return base.to_string();
  }

  let mut counter = 2;
  loop {
    let candidate = append_suffix(base, counter);
    if let Some(id) = current {
      if id == candidate {
        return candidate;
      }
    }
    if !dir.join(&candidate).exists() {
      return candidate;
    }
    counter += 1;
  }
}

fn append_suffix(base: &str, counter: usize) -> String {
  if let Some(stem) = base.strip_suffix(".md") {
    format!("{}_{}.md", stem, counter)
  } else {
    format!("{}_{}", base, counter)
  }
}

fn extract_prefix(file_name: &str) -> Option<String> {
  let regex = Regex::new(r"^(\d{2}-\d{2}-\d{4}_\d{2}-\d{2})--").ok()?;
  regex
    .captures(file_name)
    .map(|caps| format!("{}--", &caps[1]))
}

fn current_prefix() -> String {
  let now = Local::now();
  format!(
    "{}_{}--",
    now.format("%d-%m-%Y"),
    now.format("%H-%M")
  )
}

fn parse_date(value: &str) -> Option<NaiveDate> {
  NaiveDate::parse_from_str(value, "%d-%m-%Y").ok()
}

fn file_name(path: &Path) -> Result<String, NotesError> {
  path
    .file_name()
    .and_then(|name| name.to_str())
    .map(str::to_string)
    .ok_or_else(|| NotesError::new("Invalid file name"))
}
