pub mod automation;
pub mod bookshelf;
pub mod config;
pub mod db;
pub mod domain;
pub mod favorites;
pub mod mangacon;

use crate::{
    automation::AutomationRunStatus,
    bookshelf::{match_local_manga, scan_bookshelf},
    config::AppConfig,
    db::CompanionDatabase,
    domain::ImportSummary,
    favorites::import_mangacon_favorites,
    mangacon::{
        capture::{scan_mangacon_badges as scan_mangacon_badges_inner, MangaConBadgeScanResult},
        navigation::{
            open_favorites_from_home as open_mangacon_favorites_inner,
            open_first_badged_comic_from_favorites as open_first_updated_comic_inner,
            scan_detail_updates_with_scroll as scan_detail_updates_inner,
            scan_favorites_updates_with_scroll as scan_favorites_updates_inner,
            trigger_first_detail_update_download as trigger_first_detail_update_download_inner,
            DetailUpdateScanResult, FavoritesUpdateScanResult, OpenComicResult,
            OpenFavoritesResult, TriggerDetailDownloadResult,
        },
        process::{
            launch_mangacon as launch_mangacon_process,
            restart_mangacon as restart_mangacon_process, LaunchResult,
        },
        window::MangaConWindow,
    },
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

#[tauri::command]
fn find_mangacon_windows() -> Vec<MangaConWindow> {
    mangacon::window::find_mangacon_windows()
}

#[tauri::command]
fn launch_mangacon(executable_path: String) -> Result<LaunchResult, String> {
    launch_mangacon_process(executable_path).map_err(|err| err.to_string())
}

#[tauri::command]
fn restart_mangacon(executable_path: String) -> Result<LaunchResult, String> {
    restart_mangacon_process(executable_path).map_err(|err| err.to_string())
}

#[tauri::command]
fn get_automation_status() -> AutomationRunStatus {
    AutomationRunStatus::waiting_refresh(0, 0)
}

#[tauri::command]
fn scan_mangacon_badges() -> Result<MangaConBadgeScanResult, String> {
    scan_mangacon_badges_inner().map_err(|err| err.to_string())
}

#[tauri::command]
fn open_mangacon_favorites() -> Result<OpenFavoritesResult, String> {
    open_mangacon_favorites_inner().map_err(|err| err.to_string())
}

#[tauri::command]
fn open_first_updated_comic() -> Result<OpenComicResult, String> {
    open_first_updated_comic_inner().map_err(|err| err.to_string())
}

#[tauri::command]
fn scan_detail_updates() -> Result<DetailUpdateScanResult, String> {
    scan_detail_updates_inner().map_err(|err| err.to_string())
}

#[tauri::command]
fn scan_favorites_updates() -> Result<FavoritesUpdateScanResult, String> {
    scan_favorites_updates_inner().map_err(|err| err.to_string())
}

#[tauri::command]
fn trigger_first_detail_update_download() -> Result<TriggerDetailDownloadResult, String> {
    trigger_first_detail_update_download_inner().map_err(|err| err.to_string())
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

    let mut comics = import_mangacon_favorites(favorites_path)?;
    let library = scan_bookshelf(bookshelf_root)?;
    let db = CompanionDatabase::open(database_path)?;
    db.migrate()?;

    for comic in &mut comics {
        if let Some(local_match) = match_local_manga(comic.title(), &library) {
            comic.local_path = Some(local_match.directory);
            comic.chapter_count = local_match.chapter_count;
            comic.image_count = local_match.image_count;
            comic.scan_status = domain::ScanStatus::Matched;
        } else {
            comic.scan_status = domain::ScanStatus::Missing;
        }
        db.upsert_comic(comic)?;
    }

    let matched = comics
        .iter()
        .filter(|record| record.local_path.is_some())
        .count();
    Ok(ImportSummary {
        imported: comics.len(),
        matched,
        favorites: comics,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            import_favorites,
            find_mangacon_windows,
            launch_mangacon,
            restart_mangacon,
            get_automation_status,
            scan_mangacon_badges,
            open_mangacon_favorites,
            open_first_updated_comic,
            scan_favorites_updates,
            scan_detail_updates,
            trigger_first_detail_update_download
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn import_favorites_summary_exposes_source_fields() {
        let temp = tempfile::tempdir().expect("tempdir");
        let favorites_path = temp.path().join("favorites.json");
        let db_path = temp.path().join("state.sqlite");
        fs::write(
            &favorites_path,
            r#"{"favorites":[{"location":"若世界處於黑夜","name":"若世界處於黑夜","tags":["むちまろ"],"uri":"cp:ruoshijiechuyuheiye"}]}"#,
        )
        .expect("favorites fixture");

        let summary = import_favorites_inner(
            Some(favorites_path.display().to_string()),
            Some(temp.path().display().to_string()),
            db_path.display().to_string(),
        )
        .expect("import summary");

        assert_eq!(summary.imported, 1);
        assert_eq!(summary.favorites[0].source_uri, "cp:ruoshijiechuyuheiye");
        assert_eq!(summary.favorites[0].source_scheme.as_deref(), Some("cp"));
    }

    #[test]
    fn import_favorites_summary_contains_only_current_import_batch() {
        let temp = tempfile::tempdir().expect("tempdir");
        let old_path = temp.path().join("old.json");
        let new_path = temp.path().join("new.json");
        let db_path = temp.path().join("state.sqlite");
        fs::write(
            &old_path,
            r#"{"favorites":[{"location":"旧记录","name":"旧记录","tags":["old"],"uri":"cp:old"}]}"#,
        )
        .expect("old fixture");
        fs::write(
            &new_path,
            r#"{"favorites":[{"location":"新记录","name":"新记录","tags":["new"],"uri":"cp:new"}]}"#,
        )
        .expect("new fixture");

        import_favorites_inner(
            Some(old_path.display().to_string()),
            Some(temp.path().display().to_string()),
            db_path.display().to_string(),
        )
        .expect("old import");
        let summary = import_favorites_inner(
            Some(new_path.display().to_string()),
            Some(temp.path().display().to_string()),
            db_path.display().to_string(),
        )
        .expect("new import");

        assert_eq!(summary.imported, 1);
        assert_eq!(summary.matched, 0);
        assert_eq!(summary.favorites.len(), 1);
        assert_eq!(summary.favorites[0].source_uri, "cp:new");
    }
}
