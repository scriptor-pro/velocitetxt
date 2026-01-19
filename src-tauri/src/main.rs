#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use tauri::{CustomMenuItem, Manager, Menu, PhysicalPosition, PhysicalSize, Position, Size, Submenu};

mod commands;
mod models;
mod services;

struct WatcherState(Mutex<Option<services::watcher::WatcherHandle>>);

impl Default for WatcherState {
  fn default() -> Self {
    Self(Mutex::new(None))
  }
}

fn main() {
  let help_item = CustomMenuItem::new("help", "Help");
  let settings_item = CustomMenuItem::new("settings", "Settings");
  let help_menu = Submenu::new("Help", Menu::new().add_item(help_item));
  let settings_menu = Submenu::new("Settings", Menu::new().add_item(settings_item));
  let menu = Menu::new().add_submenu(settings_menu).add_submenu(help_menu);

  tauri::Builder::default()
    .menu(menu)
    .on_menu_event(|event| match event.menu_item_id() {
      "help" => {
        let _ = event.window().emit("menu://help", ());
      }
      "settings" => {
        let _ = event.window().emit("menu://settings", ());
      }
      _ => {}
    })
    .manage(WatcherState::default())
    .setup(|app| {
      if let Err(error) = services::indexer::rebuild_index() {
        eprintln!("Index rebuild failed: {}", error);
      }

      if let Err(error) = apply_window_layout(app) {
        eprintln!("Window layout failed: {}", error);
      }

      match services::watcher::start_watching() {
        Ok(handle) => {
          if let Ok(mut state) = app.state::<WatcherState>().0.lock() {
            *state = Some(handle);
          }
        }
        Err(error) => {
          eprintln!("Watcher failed: {}", error);
        }
      }

      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::notes::list_notes,
      commands::notes::get_note,
      commands::notes::create_note,
      commands::notes::save_note,
      commands::notes::archive_note,
      commands::indexer::rebuild_index,
      commands::search::search_notes
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

fn apply_window_layout(app: &tauri::App) -> Result<(), String> {
  let window = app
    .get_window("main")
    .ok_or_else(|| "Main window not found".to_string())?;
  let monitor = window
    .primary_monitor()
    .map_err(|error| format!("Monitor lookup failed: {}", error))?;
  let Some(monitor) = monitor else {
    return Ok(());
  };

  let size = monitor.size();
  let screen_width = size.width as f64;
  let screen_height = size.height as f64;

  let target_height = (screen_height * 0.8).round();
  let target_width = (screen_width * 0.8).round();

  let width = target_width.max(400.0).min(screen_width * 0.9) as u32;
  let height = target_height.max(600.0).min(screen_height * 0.9) as u32;

  let x = ((screen_width - width as f64) / 2.0).round().max(0.0) as i32;
  let y = (screen_height * 0.1).round().max(0.0) as i32;

  window
    .set_size(Size::Physical(PhysicalSize { width, height }))
    .map_err(|error| format!("Window size failed: {}", error))?;
  window
    .set_position(Position::Physical(PhysicalPosition { x, y }))
    .map_err(|error| format!("Window position failed: {}", error))?;

  Ok(())
}
