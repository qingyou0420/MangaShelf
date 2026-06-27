pub mod bookshelf;
pub mod config;
pub mod db;
pub mod domain;
pub mod favorites;

use crate::{
    bookshelf::{match_local_manga, scan_bookshelf},
    config::AppConfig,
    db::CompanionDatabase,
    domain::ImportSummary,
    favorites::import_mangacon_favorites,
};
use std::path::PathBuf;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn import_favorites(
    favorites_json_path: Option<String>,
    bookshelf_root: Option<String>,
    database_path: String,
) -> Result<ImportSummary, String> {
    import_favorites_inner(favorites_json_path, bookshelf_root, database_path)
        .map_err(|err| err.to_string())
}

fn import_favorites_inner(
    favorites_json_path: Option<String>,
    bookshelf_root: Option<String>,
    database_path: String,
) -> anyhow::Result<ImportSummary> {
    let defaults = AppConfig::default();
    let favorites_path = favorites_json_path
        .map(PathBuf::from)
        .unwrap_or(defaults.mangacon_favorites_json);
    let bookshelf_root = bookshelf_root
        .map(PathBuf::from)
        .unwrap_or(defaults.bookshelf_root);

    let favorites = import_mangacon_favorites(favorites_path)?;
    let library = scan_bookshelf(bookshelf_root)?;
    let db = CompanionDatabase::open(database_path)?;
    db.migrate()?;

    for favorite in &favorites {
        let local_match = match_local_manga(&favorite.title, &library);
        db.upsert_favorite(favorite, local_match.as_ref())?;
    }

    let records = db.list_favorites()?;
    let matched = records
        .iter()
        .filter(|record| record.local_match.is_some())
        .count();
    Ok(ImportSummary {
        imported: favorites.len(),
        matched,
        favorites: records,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, import_favorites])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
