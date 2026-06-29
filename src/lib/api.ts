import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AutomationRunStatus,
  DetailUpdateScanResult,
  EnsureMangaConRunningResult,
  FavoriteUpdateRecoveryEvent,
  FavoritesUpdateScanResult,
  ImportFavoritesResult,
  LaunchMangaConResult,
  LocalChapter,
  MangaConFavorite,
  MangaConBadgeScanResult,
  MangaConTaskStatus,
  MangaConWindow,
  OpenComicResult,
  OpenFavoritesResult,
  QueueMangaConUpdatesResult,
  RepairMangaConFailedTasksResult,
  RecoveringFavoriteUpdateResult,
  SyncBookshelfMatchesResult,
  TriggerDetailDownloadBatchResult,
  TriggerDetailDownloadResult,
  TriggerFavoriteUpdateBatchResult,
  TriggerNextFavoriteUpdateDownloadResult,
} from "./types";

export const FAVORITE_UPDATE_RECOVERY_EVENT = "favorite-update-recovery-event";

export interface ImportFavoritesOptions {
  favoritesJsonPath?: string;
  bookshelfRoot?: string;
  databasePath: string;
}

export type ImportSummary = ImportFavoritesResult;

export interface SyncBookshelfMatchesOptions {
  bookshelfRoot: string;
  databasePath: string;
  mangaConDatabasePath: string;
}

export interface LoadImportedComicsOptions {
  databasePath: string;
}

export interface LaunchMangaConOptions {
  executablePath: string;
}

export interface ScanLocalChaptersOptions {
  comicId: string;
  comicDirectory: string;
}

export interface ListChapterPagesOptions {
  chapterPath: string;
}

export interface TriggerFavoriteUpdateBatchOptions {
  maxUpdates?: number;
}

export interface TriggerAllFavoriteUpdatesOptions {
  maxComics?: number;
}

export interface TriggerAllFavoriteUpdatesWithRecoveryOptions {
  executablePath: string;
  maxComics?: number;
  maxRestarts?: number;
}

export interface QueueMangaConUpdatesOptions {
  mangaConDatabasePath: string;
  executablePath: string;
  maxUpdates?: number;
}

export interface GetMangaConTaskStatusOptions {
  mangaConDatabasePath: string;
}

export interface RepairMangaConFailedTasksOptions {
  mangaConDatabasePath: string;
  executablePath: string;
  maxTasks?: number;
}

export interface TriggerDetailUpdateBatchOptions {
  maxChapters?: number;
}

export function importFavorites(
  options: ImportFavoritesOptions,
): Promise<ImportSummary> {
  return invoke<ImportSummary>("import_favorites", {
    favoritesJsonPath: options.favoritesJsonPath,
    bookshelfRoot: options.bookshelfRoot,
    databasePath: options.databasePath,
  });
}

export function syncBookshelfMatches(
  options: SyncBookshelfMatchesOptions,
): Promise<SyncBookshelfMatchesResult> {
  return invoke<SyncBookshelfMatchesResult>("sync_bookshelf_matches", {
    bookshelfRoot: options.bookshelfRoot,
    databasePath: options.databasePath,
    mangaConDatabasePath: options.mangaConDatabasePath,
  });
}

export function loadImportedComics(
  options: LoadImportedComicsOptions,
): Promise<MangaConFavorite[]> {
  return invoke<MangaConFavorite[]>("load_imported_comics", {
    databasePath: options.databasePath,
  });
}

export function scanLocalChapters(
  options: ScanLocalChaptersOptions,
): Promise<LocalChapter[]> {
  return invoke<LocalChapter[]>("scan_local_chapters", {
    comicId: options.comicId,
    comicDirectory: options.comicDirectory,
  });
}

export function listChapterPages(
  options: ListChapterPagesOptions,
): Promise<string[]> {
  return invoke<string[]>("list_chapter_pages", {
    chapterPath: options.chapterPath,
  });
}

export function findMangaConWindows(): Promise<MangaConWindow[]> {
  return invoke<MangaConWindow[]>("find_mangacon_windows");
}

export function launchMangaCon(
  options: LaunchMangaConOptions,
): Promise<LaunchMangaConResult> {
  return invoke<LaunchMangaConResult>("launch_mangacon", {
    executablePath: options.executablePath,
  });
}

export function ensureMangaConRunning(
  options: LaunchMangaConOptions,
): Promise<EnsureMangaConRunningResult> {
  return invoke<EnsureMangaConRunningResult>("ensure_mangacon_running", {
    executablePath: options.executablePath,
  });
}

export function restartMangaCon(
  options: LaunchMangaConOptions,
): Promise<LaunchMangaConResult> {
  return invoke<LaunchMangaConResult>("restart_mangacon", {
    executablePath: options.executablePath,
  });
}

export function getAutomationStatus(): Promise<AutomationRunStatus> {
  return invoke<AutomationRunStatus>("get_automation_status");
}

export function scanMangaConBadges(): Promise<MangaConBadgeScanResult> {
  return invoke<MangaConBadgeScanResult>("scan_mangacon_badges");
}

export function openMangaConFavorites(): Promise<OpenFavoritesResult> {
  return invoke<OpenFavoritesResult>("open_mangacon_favorites");
}

export function openFirstUpdatedComic(): Promise<OpenComicResult> {
  return invoke<OpenComicResult>("open_first_updated_comic");
}

export function scanDetailUpdates(): Promise<DetailUpdateScanResult> {
  return invoke<DetailUpdateScanResult>("scan_detail_updates");
}

export function scanFavoritesUpdates(): Promise<FavoritesUpdateScanResult> {
  return invoke<FavoritesUpdateScanResult>("scan_favorites_updates");
}

export function triggerFirstDetailUpdateDownload(): Promise<TriggerDetailDownloadResult> {
  return invoke<TriggerDetailDownloadResult>("trigger_first_detail_update_download");
}

export function triggerDetailUpdateDownloadBatch(
  options: TriggerDetailUpdateBatchOptions = {},
): Promise<TriggerDetailDownloadBatchResult> {
  return invoke<TriggerDetailDownloadBatchResult>(
    "trigger_detail_update_download_batch",
    {
      maxChapters: options.maxChapters,
    },
  );
}

export function triggerNextFavoriteUpdateDownload(): Promise<TriggerNextFavoriteUpdateDownloadResult> {
  return invoke<TriggerNextFavoriteUpdateDownloadResult>(
    "trigger_next_favorite_update_download",
  );
}

export function triggerFavoriteUpdateBatch(
  options: TriggerFavoriteUpdateBatchOptions = {},
): Promise<TriggerFavoriteUpdateBatchResult> {
  return invoke<TriggerFavoriteUpdateBatchResult>(
    "trigger_favorite_update_batch",
    {
      maxUpdates: options.maxUpdates,
    },
  );
}

export function triggerAllFavoriteUpdates(
  options: TriggerAllFavoriteUpdatesOptions = {},
): Promise<TriggerFavoriteUpdateBatchResult> {
  return invoke<TriggerFavoriteUpdateBatchResult>("trigger_all_favorite_updates", {
    maxComics: options.maxComics,
  });
}

export function triggerAllFavoriteUpdatesWithRecovery(
  options: TriggerAllFavoriteUpdatesWithRecoveryOptions,
): Promise<RecoveringFavoriteUpdateResult> {
  return invoke<RecoveringFavoriteUpdateResult>(
    "trigger_all_favorite_updates_with_recovery",
    {
      executablePath: options.executablePath,
      maxComics: options.maxComics,
      maxRestarts: options.maxRestarts,
    },
  );
}

export function queueMangaConUpdates(
  options: QueueMangaConUpdatesOptions,
): Promise<QueueMangaConUpdatesResult> {
  return invoke<QueueMangaConUpdatesResult>("queue_mangacon_updates", {
    mangaConDatabasePath: options.mangaConDatabasePath,
    executablePath: options.executablePath,
    maxUpdates: options.maxUpdates,
  });
}

export function getMangaConTaskStatus(
  options: GetMangaConTaskStatusOptions,
): Promise<MangaConTaskStatus> {
  return invoke<MangaConTaskStatus>("get_mangacon_task_status", {
    mangaConDatabasePath: options.mangaConDatabasePath,
  });
}

export function repairMangaConFailedTasks(
  options: RepairMangaConFailedTasksOptions,
): Promise<RepairMangaConFailedTasksResult> {
  return invoke<RepairMangaConFailedTasksResult>(
    "repair_mangacon_failed_tasks",
    {
      mangaConDatabasePath: options.mangaConDatabasePath,
      executablePath: options.executablePath,
      maxTasks: options.maxTasks,
    },
  );
}

export function listenFavoriteUpdateRecoveryEvents(
  handler: (event: FavoriteUpdateRecoveryEvent) => void,
): Promise<UnlistenFn> {
  return listen<FavoriteUpdateRecoveryEvent>(
    FAVORITE_UPDATE_RECOVERY_EVENT,
    (event) => handler(event.payload),
  );
}
