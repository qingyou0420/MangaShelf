pub mod automation;
pub mod bookshelf;
pub mod config;
pub mod db;
pub mod domain;
pub mod favorites;
pub mod mangacon;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

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
            favorite_update_all_limit, open_favorites_from_home as open_mangacon_favorites_inner,
            open_first_badged_comic_from_favorites as open_first_updated_comic_inner,
            scan_detail_updates_with_scroll as scan_detail_updates_inner,
            scan_favorites_updates_with_scroll as scan_favorites_updates_inner,
            trigger_all_favorite_updates as trigger_all_favorite_updates_inner,
            trigger_all_favorite_updates_with_progress,
            trigger_detail_update_download_batch as trigger_detail_update_download_batch_inner,
            trigger_favorite_update_batch as trigger_favorite_update_batch_inner,
            trigger_first_detail_update_download as trigger_first_detail_update_download_inner,
            trigger_next_favorite_update_download as trigger_next_favorite_update_download_inner,
            DetailUpdateScanResult, FavoriteUpdateProgress, FavoriteUpdateProgressKind,
            FavoritesUpdateScanResult, OpenComicResult, OpenFavoritesResult,
            TriggerDetailDownloadBatchResult, TriggerDetailDownloadResult,
            TriggerFavoriteUpdateBatchResult, TriggerNextFavoriteUpdateDownloadResult,
        },
        process::{
            launch_mangacon as launch_mangacon_process,
            restart_mangacon as restart_mangacon_process, LaunchResult,
        },
        window::MangaConWindow,
    },
};
use std::{path::PathBuf, thread, time::Duration};

const FAVORITE_UPDATE_RECOVERY_DEFAULT_RESTARTS: u32 = 2;
const FAVORITE_UPDATE_RECOVERY_MAX_RESTARTS: u32 = 5;
const MANGACON_RECOVERY_REFRESH_WAIT_MS: u64 = 12_000;
const FAVORITE_UPDATE_RECOVERY_EVENT: &str = "favorite-update-recovery-event";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FavoriteUpdateRecoveryStoppedReason {
    Completed,
    RestartLimitReached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FavoriteUpdateRecoveryEventKind {
    Started,
    RunCompleted,
    ComicDownloaded,
    ComicSkipped,
    Error,
    Restarted,
    Completed,
    RestartLimitReached,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteUpdateRecoveryEvent {
    pub kind: FavoriteUpdateRecoveryEventKind,
    pub message: String,
    pub processed: u32,
    pub downloaded_chapters: u32,
    pub skipped_count: u32,
    pub restarts: u32,
}

impl FavoriteUpdateRecoveryEvent {
    fn started(requested_limit: u32) -> Self {
        Self {
            kind: FavoriteUpdateRecoveryEventKind::Started,
            message: format!("开始自动恢复长跑，目标 {requested_limit} 本"),
            processed: 0,
            downloaded_chapters: 0,
            skipped_count: 0,
            restarts: 0,
        }
    }

    fn run_completed(
        run_number: u32,
        run: &TriggerFavoriteUpdateBatchResult,
        restarts: u32,
    ) -> Self {
        Self {
            kind: FavoriteUpdateRecoveryEventKind::RunCompleted,
            message: format!("第 {run_number} 轮完成"),
            processed: run.processed,
            downloaded_chapters: run.downloaded_chapters,
            skipped_count: run.skipped.len() as u32,
            restarts,
        }
    }

    fn comic_downloaded(
        processed: u32,
        downloaded_chapters: u32,
        skipped_count: u32,
        downloaded_this_comic: u32,
    ) -> Self {
        Self {
            kind: FavoriteUpdateRecoveryEventKind::ComicDownloaded,
            message: format!("第 {processed} 本已交给漫画控，下载 {downloaded_this_comic} 话"),
            processed,
            downloaded_chapters,
            skipped_count,
            restarts: 0,
        }
    }

    fn comic_skipped(processed: u32, downloaded_chapters: u32, skipped_count: u32) -> Self {
        Self {
            kind: FavoriteUpdateRecoveryEventKind::ComicSkipped,
            message: "跳过 1 本：详情页没有更新红点".to_string(),
            processed,
            downloaded_chapters,
            skipped_count,
            restarts: 0,
        }
    }

    fn error(
        message: String,
        processed: u32,
        downloaded_chapters: u32,
        skipped_count: u32,
        restarts: u32,
    ) -> Self {
        Self {
            kind: FavoriteUpdateRecoveryEventKind::Error,
            message,
            processed,
            downloaded_chapters,
            skipped_count,
            restarts,
        }
    }

    fn restarted(restarts: u32, max_restarts: u32) -> Self {
        Self {
            kind: FavoriteUpdateRecoveryEventKind::Restarted,
            message: format!("漫画控已重启，等待红点刷新（{restarts}/{max_restarts}）"),
            processed: 0,
            downloaded_chapters: 0,
            skipped_count: 0,
            restarts,
        }
    }

    fn completed(
        processed: u32,
        downloaded_chapters: u32,
        skipped_count: u32,
        restarts: u32,
    ) -> Self {
        Self {
            kind: FavoriteUpdateRecoveryEventKind::Completed,
            message: "自动恢复长跑完成".to_string(),
            processed,
            downloaded_chapters,
            skipped_count,
            restarts,
        }
    }

    fn restart_limit_reached(
        processed: u32,
        downloaded_chapters: u32,
        skipped_count: u32,
        restarts: u32,
    ) -> Self {
        Self {
            kind: FavoriteUpdateRecoveryEventKind::RestartLimitReached,
            message: "已达到自动重启上限".to_string(),
            processed,
            downloaded_chapters,
            skipped_count,
            restarts,
        }
    }

    fn with_totals(
        mut self,
        processed: u32,
        downloaded_chapters: u32,
        skipped_count: u32,
    ) -> Self {
        self.processed = processed;
        self.downloaded_chapters = downloaded_chapters;
        self.skipped_count = skipped_count;
        self
    }

    fn with_restarts(mut self, restarts: u32) -> Self {
        self.restarts = restarts;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveringFavoriteUpdateResult {
    pub requested_limit: u32,
    pub max_restarts: u32,
    pub restarts: u32,
    pub processed: u32,
    pub downloaded_chapters: u32,
    pub skipped_count: u32,
    pub stopped_reason: FavoriteUpdateRecoveryStoppedReason,
    pub last_error: Option<String>,
    pub events: Vec<FavoriteUpdateRecoveryEvent>,
    pub runs: Vec<TriggerFavoriteUpdateBatchResult>,
}

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

#[tauri::command]
fn trigger_detail_update_download_batch(
    max_chapters: Option<u32>,
) -> Result<TriggerDetailDownloadBatchResult, String> {
    trigger_detail_update_download_batch_inner(max_chapters).map_err(|err| err.to_string())
}

#[tauri::command]
fn trigger_next_favorite_update_download() -> Result<TriggerNextFavoriteUpdateDownloadResult, String>
{
    trigger_next_favorite_update_download_inner().map_err(|err| err.to_string())
}

#[tauri::command]
fn trigger_favorite_update_batch(
    max_updates: Option<u32>,
) -> Result<TriggerFavoriteUpdateBatchResult, String> {
    trigger_favorite_update_batch_inner(max_updates).map_err(|err| err.to_string())
}

#[tauri::command]
fn trigger_all_favorite_updates(
    max_comics: Option<u32>,
) -> Result<TriggerFavoriteUpdateBatchResult, String> {
    trigger_all_favorite_updates_inner(max_comics).map_err(|err| err.to_string())
}

#[tauri::command]
fn trigger_all_favorite_updates_with_recovery(
    app: AppHandle,
    executable_path: String,
    max_comics: Option<u32>,
    max_restarts: Option<u32>,
) -> Result<RecoveringFavoriteUpdateResult, String> {
    let mut event_sink = |event: &FavoriteUpdateRecoveryEvent| {
        let _ = app.emit(FAVORITE_UPDATE_RECOVERY_EVENT, event);
    };
    trigger_all_favorite_updates_with_recovery_inner_with_event_sink(
        executable_path,
        max_comics,
        max_restarts,
        &mut event_sink,
    )
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

fn trigger_all_favorite_updates_with_recovery_inner_with_event_sink<F>(
    executable_path: String,
    max_comics: Option<u32>,
    max_restarts: Option<u32>,
    event_sink: &mut F,
) -> Result<RecoveringFavoriteUpdateResult, String>
where
    F: FnMut(&FavoriteUpdateRecoveryEvent),
{
    let requested_limit = favorite_update_all_limit(max_comics);
    let max_restarts = favorite_update_recovery_restart_limit(max_restarts);
    let mut restarts = 0;
    let mut last_error = None;
    let mut runs = Vec::new();
    let mut events = Vec::new();
    record_recovery_event(
        &mut events,
        event_sink,
        FavoriteUpdateRecoveryEvent::started(requested_limit),
    );

    loop {
        let run_result = {
            let current_restarts = restarts;
            let mut progress_sink = |progress| {
                record_recovery_event(
                    &mut events,
                    event_sink,
                    recovery_event_from_favorite_progress(progress, current_restarts),
                );
            };
            trigger_all_favorite_updates_with_progress(Some(requested_limit), &mut progress_sink)
        };
        match run_result {
            Ok(run) => {
                record_recovery_event(
                    &mut events,
                    event_sink,
                    FavoriteUpdateRecoveryEvent::run_completed(
                        runs.len() as u32 + 1,
                        &run,
                        restarts,
                    ),
                );
                runs.push(run);
                return Ok(recovering_favorite_update_result_with_event_sink(
                    FavoriteUpdateRecoveryResultInput {
                        requested_limit,
                        max_restarts,
                        restarts,
                        stopped_reason: FavoriteUpdateRecoveryStoppedReason::Completed,
                        last_error,
                        runs,
                        events,
                    },
                    event_sink,
                ));
            }
            Err(error) => {
                let error_message = error.to_string();
                last_error = Some(error_message.clone());
                let (processed, downloaded_chapters, skipped_count) =
                    favorite_update_run_totals(&runs);
                record_recovery_event(
                    &mut events,
                    event_sink,
                    FavoriteUpdateRecoveryEvent::error(
                        error_message,
                        processed,
                        downloaded_chapters,
                        skipped_count,
                        restarts,
                    ),
                );

                if restarts >= max_restarts {
                    return Ok(recovering_favorite_update_result_with_event_sink(
                        FavoriteUpdateRecoveryResultInput {
                            requested_limit,
                            max_restarts,
                            restarts,
                            stopped_reason:
                                FavoriteUpdateRecoveryStoppedReason::RestartLimitReached,
                            last_error,
                            runs,
                            events,
                        },
                        event_sink,
                    ));
                }

                restart_mangacon_process(&executable_path).map_err(|err| err.to_string())?;
                restarts += 1;
                record_recovery_event(
                    &mut events,
                    event_sink,
                    FavoriteUpdateRecoveryEvent::restarted(restarts, max_restarts).with_totals(
                        processed,
                        downloaded_chapters,
                        skipped_count,
                    ),
                );
                thread::sleep(Duration::from_millis(MANGACON_RECOVERY_REFRESH_WAIT_MS));
            }
        }
    }
}

struct FavoriteUpdateRecoveryResultInput {
    requested_limit: u32,
    max_restarts: u32,
    restarts: u32,
    stopped_reason: FavoriteUpdateRecoveryStoppedReason,
    last_error: Option<String>,
    runs: Vec<TriggerFavoriteUpdateBatchResult>,
    events: Vec<FavoriteUpdateRecoveryEvent>,
}

#[cfg(test)]
fn recovering_favorite_update_result(
    input: FavoriteUpdateRecoveryResultInput,
) -> RecoveringFavoriteUpdateResult {
    let mut event_sink = |_event: &FavoriteUpdateRecoveryEvent| {};
    recovering_favorite_update_result_with_event_sink(input, &mut event_sink)
}

fn recovering_favorite_update_result_with_event_sink<F>(
    input: FavoriteUpdateRecoveryResultInput,
    event_sink: &mut F,
) -> RecoveringFavoriteUpdateResult
where
    F: FnMut(&FavoriteUpdateRecoveryEvent),
{
    let FavoriteUpdateRecoveryResultInput {
        requested_limit,
        max_restarts,
        restarts,
        stopped_reason,
        last_error,
        runs,
        mut events,
    } = input;
    let (processed, downloaded_chapters, skipped_count) = favorite_update_run_totals(&runs);

    match stopped_reason {
        FavoriteUpdateRecoveryStoppedReason::Completed => {
            record_recovery_event(
                &mut events,
                event_sink,
                FavoriteUpdateRecoveryEvent::completed(
                    processed,
                    downloaded_chapters,
                    skipped_count,
                    restarts,
                ),
            );
        }
        FavoriteUpdateRecoveryStoppedReason::RestartLimitReached => {
            record_recovery_event(
                &mut events,
                event_sink,
                FavoriteUpdateRecoveryEvent::restart_limit_reached(
                    processed,
                    downloaded_chapters,
                    skipped_count,
                    restarts,
                ),
            );
        }
    }

    RecoveringFavoriteUpdateResult {
        requested_limit,
        max_restarts,
        restarts,
        processed,
        downloaded_chapters,
        skipped_count,
        stopped_reason,
        last_error,
        events,
        runs,
    }
}

fn record_recovery_event<F>(
    events: &mut Vec<FavoriteUpdateRecoveryEvent>,
    event_sink: &mut F,
    event: FavoriteUpdateRecoveryEvent,
) where
    F: FnMut(&FavoriteUpdateRecoveryEvent),
{
    event_sink(&event);
    events.push(event);
}

fn recovery_event_from_favorite_progress(
    progress: FavoriteUpdateProgress,
    restarts: u32,
) -> FavoriteUpdateRecoveryEvent {
    match progress.kind {
        FavoriteUpdateProgressKind::ComicDownloaded => FavoriteUpdateRecoveryEvent::comic_downloaded(
            progress.processed,
            progress.downloaded_chapters,
            progress.skipped_count,
            progress.chapter_delta,
        )
        .with_restarts(restarts),
        FavoriteUpdateProgressKind::ComicSkipped => FavoriteUpdateRecoveryEvent::comic_skipped(
            progress.processed,
            progress.downloaded_chapters,
            progress.skipped_count,
        )
        .with_restarts(restarts),
    }
}

fn favorite_update_run_totals(runs: &[TriggerFavoriteUpdateBatchResult]) -> (u32, u32, u32) {
    (
        runs.iter().map(|run| run.processed).sum(),
        runs.iter().map(|run| run.downloaded_chapters).sum(),
        runs.iter().map(|run| run.skipped.len() as u32).sum(),
    )
}

fn favorite_update_recovery_restart_limit(max_restarts: Option<u32>) -> u32 {
    max_restarts
        .unwrap_or(FAVORITE_UPDATE_RECOVERY_DEFAULT_RESTARTS)
        .clamp(0, FAVORITE_UPDATE_RECOVERY_MAX_RESTARTS)
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
            trigger_first_detail_update_download,
            trigger_detail_update_download_batch,
            trigger_next_favorite_update_download,
            trigger_favorite_update_batch,
            trigger_all_favorite_updates,
            trigger_all_favorite_updates_with_recovery
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
    fn favorite_update_recovery_restart_limit_is_safe_and_bounded() {
        assert_eq!(favorite_update_recovery_restart_limit(None), 2);
        assert_eq!(favorite_update_recovery_restart_limit(Some(0)), 0);
        assert_eq!(favorite_update_recovery_restart_limit(Some(4)), 4);
        assert_eq!(favorite_update_recovery_restart_limit(Some(99)), 5);
    }

    #[test]
    fn recovering_favorite_update_result_keeps_long_run_events() {
        let run = TriggerFavoriteUpdateBatchResult {
            requested_limit: 500,
            processed: 2,
            downloaded_chapters: 3,
            stopped_reason: mangacon::navigation::FavoriteUpdateBatchStoppedReason::NoUpdateBadge,
            skipped: Vec::new(),
            items: Vec::new(),
        };
        let events = vec![
            FavoriteUpdateRecoveryEvent::started(500),
            FavoriteUpdateRecoveryEvent::error("漫画控窗口无响应".to_string(), 0, 0, 0, 0),
            FavoriteUpdateRecoveryEvent::restarted(1, 2),
        ];

        let result = recovering_favorite_update_result(FavoriteUpdateRecoveryResultInput {
            requested_limit: 500,
            max_restarts: 2,
            restarts: 1,
            stopped_reason: FavoriteUpdateRecoveryStoppedReason::Completed,
            last_error: Some("漫画控窗口无响应".to_string()),
            runs: vec![run],
            events,
        });

        assert_eq!(result.events.len(), 4);
        assert_eq!(result.events[0].kind, FavoriteUpdateRecoveryEventKind::Started);
        assert_eq!(result.events[1].kind, FavoriteUpdateRecoveryEventKind::Error);
        assert_eq!(result.events[2].kind, FavoriteUpdateRecoveryEventKind::Restarted);
        assert_eq!(result.events[3].kind, FavoriteUpdateRecoveryEventKind::Completed);
        assert_eq!(result.events[3].message, "自动恢复长跑完成");
        assert_eq!(result.events[3].processed, 2);
        assert_eq!(result.events[3].downloaded_chapters, 3);
        assert_eq!(result.events[3].restarts, 1);
    }

    #[test]
    fn recording_recovery_event_pushes_event_and_notifies_sink() {
        let mut events = Vec::new();
        let mut emitted = Vec::new();

        record_recovery_event(
            &mut events,
            &mut |event| emitted.push(event.clone()),
            FavoriteUpdateRecoveryEvent::started(500),
        );

        assert_eq!(events.len(), 1);
        assert_eq!(emitted.len(), 1);
        assert_eq!(events[0], emitted[0]);
        assert_eq!(emitted[0].kind, FavoriteUpdateRecoveryEventKind::Started);
    }

    #[test]
    fn recovery_events_describe_each_downloaded_and_skipped_comic() {
        let downloaded = FavoriteUpdateRecoveryEvent::comic_downloaded(2, 7, 1, 4);
        let skipped = FavoriteUpdateRecoveryEvent::comic_skipped(2, 7, 2);

        assert_eq!(
            downloaded.kind,
            FavoriteUpdateRecoveryEventKind::ComicDownloaded
        );
        assert_eq!(downloaded.message, "第 2 本已交给漫画控，下载 4 话");
        assert_eq!(downloaded.processed, 2);
        assert_eq!(downloaded.downloaded_chapters, 7);
        assert_eq!(downloaded.skipped_count, 1);

        assert_eq!(skipped.kind, FavoriteUpdateRecoveryEventKind::ComicSkipped);
        assert_eq!(skipped.message, "跳过 1 本：详情页没有更新红点");
        assert_eq!(skipped.processed, 2);
        assert_eq!(skipped.downloaded_chapters, 7);
        assert_eq!(skipped.skipped_count, 2);
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
