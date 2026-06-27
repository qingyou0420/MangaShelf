import { invoke } from "@tauri-apps/api/core";
import type {
  AutomationRunStatus,
  DetailUpdateScanResult,
  ImportFavoritesResult,
  LaunchMangaConResult,
  MangaConBadgeScanResult,
  MangaConWindow,
  OpenComicResult,
  OpenFavoritesResult,
  TriggerDetailDownloadResult,
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

export function triggerFirstDetailUpdateDownload(): Promise<TriggerDetailDownloadResult> {
  return invoke<TriggerDetailDownloadResult>("trigger_first_detail_update_download");
}
