pub mod bookshelf;
pub mod config;
pub mod cover;
pub mod db;
pub mod domain;
pub mod library;
pub mod update;

use tauri::{Emitter, Manager, State};

use crate::{
    bookshelf::{find_first_image_page, list_chapter_pages_with_progress},
    domain::{
        CacheStats, Chapter, Comic, FitMode, LoadLibraryResult, ReadMode, ReadingDirection,
        ScanLibraryResult,
    },
    cover::cover_or_source,
    library::{
        cache_stats as cache_stats_inner, clear_extract_cache as clear_extract_cache_inner,
        cover_candidates as cover_candidates_inner, load_library as load_library_inner,
        load_or_scan_chapters, scan_library_with_progress,
    },
    update::{
        app_version, check_github_updates, download_and_install_update,
        open_path as open_path_inner, LocalUpdateCheckResult,
    },
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

struct ScanControl {
    requested: Arc<AtomicBool>,
}

struct LibraryDbState {
    inner: std::sync::Mutex<Option<(String, crate::db::LibraryDatabase)>>,
}

impl LibraryDbState {
    fn with<T>(
        &self,
        path: &str,
        callback: impl FnOnce(&crate::db::LibraryDatabase) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        let reopen = match guard.as_ref() {
            Some((open_path, _)) => open_path != path,
            None => true,
        };
        if reopen {
            let db = crate::db::LibraryDatabase::open(path)?;
            db.migrate()?;
            *guard = Some((path.to_string(), db));
        }
        callback(&guard.as_ref().expect("db").1)
    }
}

fn allow_bookshelf_assets(app: &tauri::AppHandle, root: &str) {
    let trimmed = root.trim();
    if trimmed.is_empty() {
        return;
    }
    let path = PathBuf::from(trimmed);
    let _ = app.asset_protocol_scope().allow_directory(&path, true);
}

#[tauri::command]
async fn load_library(
    app: tauri::AppHandle,
    bookshelf_root: String,
    database_path: Option<String>,
) -> Result<LoadLibraryResult, String> {
    allow_bookshelf_assets(&app, &bookshelf_root);
    tauri::async_runtime::spawn_blocking(move || {
        load_library_inner(&bookshelf_root, database_path.as_deref())
    })
    .await
    .map_err(|err| err.to_string())?
    .map_err(|err| err.to_string())
}

#[tauri::command]
async fn scan_library(
    app: tauri::AppHandle,
    control: State<'_, ScanControl>,
    bookshelf_root: String,
    database_path: Option<String>,
    extra_roots: Option<Vec<String>>,
) -> Result<ScanLibraryResult, String> {
    allow_bookshelf_assets(&app, &bookshelf_root);
    for extra in extra_roots.iter().flatten() {
        allow_bookshelf_assets(&app, extra);
    }
    control.requested.store(false, Ordering::SeqCst);
    let flag = control.requested.clone();
    let emit_app = app.clone();
    let extras = extra_roots.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        scan_library_with_progress(
            &bookshelf_root,
            database_path.as_deref(),
            &extras,
            |progress| {
                let _ = emit_app.emit("library-scan-progress", progress);
                !flag.load(Ordering::SeqCst)
            },
        )
    })
    .await
    .map_err(|err| err.to_string())?
    .map_err(|err| err.to_string())
}

#[tauri::command]
fn cancel_library_scan(control: State<'_, ScanControl>) {
    control.requested.store(true, Ordering::SeqCst);
}

#[tauri::command]
fn allow_asset_root(app: tauri::AppHandle, path: String) {
    allow_bookshelf_assets(&app, &path);
}

#[tauri::command]
async fn pick_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let picked = rfd::FileDialog::new()
            .set_title("选择书架文件夹")
            .pick_folder()
            .map(|path| path.to_string_lossy().into_owned());
        let _ = tx.send(picked);
    })
    .map_err(|err| err.to_string())?;
    rx.recv().map_err(|err| err.to_string())
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    open_path_inner(path)
}

#[tauri::command]
fn path_is_directory(path: String) -> bool {
    PathBuf::from(path).is_dir()
}

#[tauri::command]
async fn scan_local_chapters(
    comic_id: String,
    comic_directory: String,
    database_path: Option<String>,
    force: Option<bool>,
) -> Result<Vec<Chapter>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        load_or_scan_chapters(
            &comic_id,
            comic_directory,
            database_path.as_deref(),
            force.unwrap_or(false),
        )
    })
    .await
    .map_err(|err| err.to_string())?
    .map_err(|err| err.to_string())
}

#[tauri::command]
async fn list_chapter_pages(
    app: tauri::AppHandle,
    chapter_path: String,
    bookshelf_root: Option<String>,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        list_chapter_pages_with_progress(
            chapter_path,
            bookshelf_root.as_deref(),
            |progress| {
                let _ = app.emit("library-extract-progress", progress);
            },
        )
    })
    .await
    .map_err(|err| err.to_string())?
    .map_err(|err| err.to_string())
}

#[tauri::command]
async fn peek_chapter_first_page(chapter_path: String) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        find_first_image_page(PathBuf::from(chapter_path))
            .map(|page| page.map(|path| path.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|err| err.to_string())?
    .map_err(|err| err.to_string())
}

#[tauri::command]
async fn save_read_progress(
    app: tauri::AppHandle,
    database_path: String,
    comic_id: String,
    chapter_id: String,
    page: u32,
) -> Result<Option<Comic>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<LibraryDbState>();
        state
            .with(&database_path, |db| {
                db.save_read_progress(&comic_id, &chapter_id, page)
            })
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn update_comic_metadata(
    app: tauri::AppHandle,
    database_path: String,
    comic_id: String,
    name: Option<String>,
    author: Option<String>,
    tags: Option<Vec<String>>,
) -> Result<Option<Comic>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<LibraryDbState>();
        state
            .with(&database_path, |db| {
                db.update_comic_metadata(
                    &comic_id,
                    name.as_deref(),
                    author.as_deref(),
                    tags.as_deref(),
                )
            })
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn set_comic_favorite(
    app: tauri::AppHandle,
    database_path: String,
    comic_id: String,
    favorited: bool,
) -> Result<Option<Comic>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<LibraryDbState>();
        state
            .with(&database_path, |db| db.set_comic_favorite(&comic_id, favorited))
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn set_reader_prefs(
    app: tauri::AppHandle,
    database_path: String,
    comic_id: String,
    reading_direction: ReadingDirection,
    fit_mode: FitMode,
    read_mode: Option<ReadMode>,
) -> Result<Option<Comic>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<LibraryDbState>();
        state
            .with(&database_path, |db| {
                db.set_reader_prefs(
                    &comic_id,
                    reading_direction,
                    fit_mode,
                    read_mode.unwrap_or(ReadMode::Page),
                )
            })
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn clear_read_progress(
    app: tauri::AppHandle,
    database_path: String,
    comic_id: String,
) -> Result<Option<Comic>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<LibraryDbState>();
        state
            .with(&database_path, |db| db.clear_read_progress(&comic_id))
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn delete_library_comic(
    app: tauri::AppHandle,
    database_path: String,
    comic_id: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<LibraryDbState>();
        state
            .with(&database_path, |db| db.delete_comic(&comic_id))
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn list_cover_candidates(comic_directory: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || cover_candidates_inner(comic_directory))
        .await
        .map_err(|err| err.to_string())?
        .map_err(|err| err.to_string())
}

#[tauri::command]
async fn set_comic_cover(
    app: tauri::AppHandle,
    bookshelf_root: String,
    database_path: String,
    comic_id: String,
    source_path: String,
) -> Result<Option<Comic>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<LibraryDbState>();
        state
            .with(&database_path, |db| {
                let Some(mut comic) = db.get_comic(&comic_id)? else {
                    return Ok(None);
                };
                comic.cover_path = Some(cover_or_source(
                    std::path::Path::new(&bookshelf_root),
                    &comic_id,
                    std::path::PathBuf::from(source_path),
                ));
                db.upsert_comic(&comic)?;
                db.get_comic(&comic_id)
            })
            .map_err(|err| err.to_string())
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn library_cache_stats(
    bookshelf_root: String,
    extra_roots: Option<Vec<String>>,
) -> Result<CacheStats, String> {
    tauri::async_runtime::spawn_blocking(move || {
        cache_stats_inner(bookshelf_root, extra_roots.unwrap_or_default().as_slice())
    })
    .await
    .map_err(|err| err.to_string())?
    .map_err(|err| err.to_string())
}

#[tauri::command]
async fn clear_library_cache(
    bookshelf_root: String,
    extra_roots: Option<Vec<String>>,
) -> Result<CacheStats, String> {
    tauri::async_runtime::spawn_blocking(move || {
        clear_extract_cache_inner(
            bookshelf_root,
            extra_roots.unwrap_or_default().as_slice(),
            Some(0),
        )
    })
    .await
    .map_err(|err| err.to_string())?
    .map_err(|err| err.to_string())
}

#[tauri::command]
fn get_app_version() -> String {
    app_version().to_string()
}

#[tauri::command]
async fn check_local_installer_updates() -> Result<LocalUpdateCheckResult, String> {
    tauri::async_runtime::spawn_blocking(|| check_github_updates(app_version()))
        .await
        .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn open_local_installer(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        if path.starts_with("http://") || path.starts_with("https://") {
            let file_name = path
                .rsplit('/')
                .next()
                .unwrap_or("MangaShelf-setup.exe")
                .to_string();
            return download_and_install_update(&path, &file_name);
        }
        crate::update::open_local_installer(path)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[tauri::command]
async fn install_app_update(download_url: String, file_name: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        download_and_install_update(&download_url, &file_name)
    })
    .await
    .map_err(|err| err.to_string())?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(ScanControl {
                requested: Arc::new(AtomicBool::new(false)),
            });
            app.manage(LibraryDbState {
                inner: std::sync::Mutex::new(None),
            });
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_decorations(true);
                let _ = window.set_closable(true);
                let _ = window.set_resizable(true);
                let _ = window.set_minimizable(true);
                let _ = window.set_maximizable(true);
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_library,
            scan_library,
            cancel_library_scan,
            scan_local_chapters,
            list_chapter_pages,
            peek_chapter_first_page,
            save_read_progress,
            update_comic_metadata,
            set_comic_favorite,
            set_reader_prefs,
            get_app_version,
            check_local_installer_updates,
            open_local_installer,
            install_app_update,
            pick_directory,
            open_path,
            path_is_directory,
            allow_asset_root,
            delete_library_comic,
            clear_read_progress,
            list_cover_candidates,
            set_comic_cover,
            library_cache_stats,
            clear_library_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;

    #[test]
    fn default_config_is_local_bookshelf_only() {
        let config = AppConfig::default();
        assert_eq!(config.bookshelf_root.display().to_string(), r"E:\书架");
        let db = config.library_database.display().to_string();
        assert!(
            db.ends_with("manga-library.sqlite") || db.ends_with("mangacon-companion.sqlite"),
            "{db}"
        );
    }
}
