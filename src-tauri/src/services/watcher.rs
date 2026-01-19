use std::collections::HashSet;
use std::fmt;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::services::{indexer, notes_fs};

const DEBOUNCE_MS: u64 = 300;
const POLL_MS: u64 = 100;

#[derive(Debug)]
pub struct WatcherError {
  details: String,
}

impl WatcherError {
  pub fn new(details: impl Into<String>) -> Self {
    Self {
      details: details.into(),
    }
  }
}

impl fmt::Display for WatcherError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(formatter, "{}", self.details)
  }
}

impl std::error::Error for WatcherError {}

pub struct WatcherHandle {
  _watcher: RecommendedWatcher,
}

pub fn start_watching() -> Result<WatcherHandle, WatcherError> {
  let notes_dir = notes_fs::notes_dir().map_err(|error| WatcherError::new(error.to_string()))?;
  if !notes_dir.exists() {
    if let Err(error) = std::fs::create_dir_all(&notes_dir) {
      return Err(WatcherError::new(format!("Create notes dir failed: {}", error)));
    }
  }

  let (sender, receiver) = mpsc::channel();
  let mut watcher = notify::recommended_watcher(move |res| {
    let _ = sender.send(res);
  })
  .map_err(|error| WatcherError::new(format!("Watcher init failed: {}", error)))?;

  watcher
    .watch(&notes_dir, RecursiveMode::NonRecursive)
    .map_err(|error| WatcherError::new(format!("Watch failed: {}", error)))?;

  thread::spawn(move || {
    let mut pending_upsert: HashSet<PathBuf> = HashSet::new();
    let mut pending_delete: HashSet<PathBuf> = HashSet::new();
    let mut last_event: Option<Instant> = None;

    loop {
      match receiver.recv_timeout(Duration::from_millis(POLL_MS)) {
        Ok(event) => {
          if let Ok(event) = event {
            queue_event(event, &mut pending_upsert, &mut pending_delete);
            last_event = Some(Instant::now());
          }
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
          if let Some(last) = last_event {
            if last.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
              flush_pending(&mut pending_upsert, &mut pending_delete);
              last_event = None;
            }
          }
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => break,
      }
    }

    flush_pending(&mut pending_upsert, &mut pending_delete);
  });

  Ok(WatcherHandle { _watcher: watcher })
}

fn queue_event(
  event: Event,
  pending_upsert: &mut HashSet<PathBuf>,
  pending_delete: &mut HashSet<PathBuf>,
) {
  match event.kind {
    EventKind::Create(_) | EventKind::Modify(_) => {
      for path in event.paths {
        pending_delete.remove(&path);
        pending_upsert.insert(path);
      }
    }
    EventKind::Remove(_) => {
      for path in event.paths {
        pending_upsert.remove(&path);
        pending_delete.insert(path);
      }
    }
    _ => {}
  }
}

fn flush_pending(pending_upsert: &mut HashSet<PathBuf>, pending_delete: &mut HashSet<PathBuf>) {
  for path in pending_upsert.drain() {
    let _ = indexer::upsert_note_by_path(&path);
  }

  for path in pending_delete.drain() {
    let _ = indexer::delete_note_by_path(&path);
  }
}
