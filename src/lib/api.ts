import { invoke } from "@tauri-apps/api/core";
import type {
  AutomationRunStatus,
  ImportFavoritesResult,
  LaunchMangaConResult,
  MangaConWindow,
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

export function getAutomationStatus(): Promise<AutomationRunStatus> {
  return invoke<AutomationRunStatus>("get_automation_status");
}
