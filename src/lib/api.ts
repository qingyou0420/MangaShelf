import { invoke } from "@tauri-apps/api/core";
import type { ImportFavoritesResult } from "./types";

export interface ImportFavoritesOptions {
  favoritesJsonPath?: string;
  bookshelfRoot?: string;
  databasePath: string;
}

export type ImportSummary = ImportFavoritesResult;

export function importFavorites(
  options: ImportFavoritesOptions,
): Promise<ImportSummary> {
  return invoke<ImportSummary>("import_favorites", {
    favoritesJsonPath: options.favoritesJsonPath,
    bookshelfRoot: options.bookshelfRoot,
    databasePath: options.databasePath,
  });
}
