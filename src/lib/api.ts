import { invoke } from "@tauri-apps/api/core";
import type {
  AutomationRunStatus,
  DetailUpdateScanResult,
  FavoritesUpdateScanResult,
  ImportFavoritesResult,
  LaunchMangaConResult,
  MangaConBadgeScanResult,
  MangaConWindow,
  OpenComicResult,
  OpenFavoritesResult,
  RecoveringFavoriteUpdateResult,
  TriggerDetailDownloadBatchResult,
  TriggerDetailDownloadResult,
  TriggerFavoriteUpdateBatchResult,
  TriggerNextFavoriteUpdateDownloadResult,
} from "./types";

export interface ImportFavoritesOptions {
  favoritesJsonPath?: string;
  bookshelfRoot?: string;
  databasePath: string;
}

export type ImportSummary = ImportFavoritesResult;

export interface LaunchMangaConOptions {
  executablePath: string;
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
