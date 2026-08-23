import { invoke } from "@tauri-apps/api/core";
import type {
  CacheStats,
  ExtractProgress,
  FitMode,
  LibraryComic,
  LibraryPaths,
  LoadLibraryResult,
  LocalChapter,
  LocalUpdateCheckResult,
  ReadMode,
  ReadingDirection,
  ScanLibraryResult,
  ScanProgress,
} from "./types";

export function loadLibrary(paths: LibraryPaths): Promise<LoadLibraryResult> {
  return invoke<LoadLibraryResult>("load_library", {
    bookshelfRoot: paths.bookshelfRoot,
    databasePath: paths.databasePath,
  });
}

export function scanLibrary(paths: LibraryPaths): Promise<ScanLibraryResult> {
  return invoke<ScanLibraryResult>("scan_library", {
    bookshelfRoot: paths.bookshelfRoot,
    databasePath: paths.databasePath,
    extraRoots: paths.extraRoots ?? [],
  });
}

export function cancelLibraryScan(): Promise<void> {
  return invoke<void>("cancel_library_scan");
}

export function scanLocalChapters(options: {
  comicId: string;
  comicDirectory: string;
  databasePath?: string;
  force?: boolean;
}): Promise<LocalChapter[]> {
  return invoke<LocalChapter[]>("scan_local_chapters", {
    comicId: options.comicId,
    comicDirectory: options.comicDirectory,
    databasePath: options.databasePath,
    force: options.force ?? false,
  });
}

export function listChapterPages(options: {
  chapterPath: string;
  bookshelfRoot?: string;
}): Promise<string[]> {
  return invoke<string[]>("list_chapter_pages", {
    chapterPath: options.chapterPath,
    bookshelfRoot: options.bookshelfRoot,
  });
}

export function peekChapterFirstPage(chapterPath: string): Promise<string | null> {
  return invoke<string | null>("peek_chapter_first_page", { chapterPath });
}

export function saveReadProgress(options: {
  databasePath: string;
  comicId: string;
  chapterId: string;
  page: number;
}): Promise<LibraryComic | null> {
  return invoke<LibraryComic | null>("save_read_progress", {
    databasePath: options.databasePath,
    comicId: options.comicId,
    chapterId: options.chapterId,
    page: options.page,
  });
}

export function updateComicMetadata(options: {
  databasePath: string;
  comicId: string;
  name?: string;
  author?: string;
  tags?: string[];
}): Promise<LibraryComic | null> {
  return invoke<LibraryComic | null>("update_comic_metadata", {
    databasePath: options.databasePath,
    comicId: options.comicId,
    name: options.name,
    author: options.author,
    tags: options.tags,
  });
}

export function setComicFavorite(options: {
  databasePath: string;
  comicId: string;
  favorited: boolean;
}): Promise<LibraryComic | null> {
  return invoke<LibraryComic | null>("set_comic_favorite", {
    databasePath: options.databasePath,
    comicId: options.comicId,
    favorited: options.favorited,
  });
}

export function setReaderPrefs(options: {
  databasePath: string;
  comicId: string;
  readingDirection: ReadingDirection;
  fitMode: FitMode;
  readMode?: ReadMode;
}): Promise<LibraryComic | null> {
  return invoke<LibraryComic | null>("set_reader_prefs", {
    databasePath: options.databasePath,
    comicId: options.comicId,
    readingDirection: options.readingDirection,
    fitMode: options.fitMode,
    readMode: options.readMode ?? "page",
  });
}

export function clearReadProgress(options: {
  databasePath: string;
  comicId: string;
}): Promise<LibraryComic | null> {
  return invoke<LibraryComic | null>("clear_read_progress", {
    databasePath: options.databasePath,
    comicId: options.comicId,
  });
}

export function pathIsDirectory(path: string): Promise<boolean> {
  return invoke<boolean>("path_is_directory", { path });
}

export function deleteLibraryComic(options: {
  databasePath: string;
  comicId: string;
}): Promise<void> {
  return invoke<void>("delete_library_comic", {
    databasePath: options.databasePath,
    comicId: options.comicId,
  });
}

export function listCoverCandidates(comicDirectory: string): Promise<string[]> {
  return invoke<string[]>("list_cover_candidates", { comicDirectory });
}

export function setComicCover(options: {
  bookshelfRoot: string;
  databasePath: string;
  comicId: string;
  sourcePath: string;
}): Promise<LibraryComic | null> {
  return invoke<LibraryComic | null>("set_comic_cover", {
    bookshelfRoot: options.bookshelfRoot,
    databasePath: options.databasePath,
    comicId: options.comicId,
    sourcePath: options.sourcePath,
  });
}

export function libraryCacheStats(options: {
  bookshelfRoot: string;
  extraRoots?: string[];
}): Promise<CacheStats> {
  return invoke<CacheStats>("library_cache_stats", {
    bookshelfRoot: options.bookshelfRoot,
    extraRoots: options.extraRoots ?? [],
  });
}

export function clearLibraryCache(options: {
  bookshelfRoot: string;
  extraRoots?: string[];
}): Promise<CacheStats> {
  return invoke<CacheStats>("clear_library_cache", {
    bookshelfRoot: options.bookshelfRoot,
    extraRoots: options.extraRoots ?? [],
  });
}

export function getAppVersion(): Promise<string> {
  return invoke<string>("get_app_version");
}

export function checkLocalInstallerUpdates(_options?: {
  searchPath?: string;
}): Promise<LocalUpdateCheckResult> {
  return invoke<LocalUpdateCheckResult>("check_local_installer_updates");
}

export function openLocalInstaller(path: string): Promise<void> {
  return invoke<void>("open_local_installer", { path });
}

export function installAppUpdate(options: {
  downloadUrl: string;
  fileName: string;
}): Promise<void> {
  return invoke<void>("install_app_update", {
    downloadUrl: options.downloadUrl,
    fileName: options.fileName,
  });
}

export function pickDirectory(): Promise<string | null> {
  return invoke<string | null>("pick_directory");
}

export function openPath(path: string): Promise<void> {
  return invoke<void>("open_path", { path });
}

export function allowAssetRoot(path: string): Promise<void> {
  return invoke<void>("allow_asset_root", { path });
}

export async function listenScanProgress(
  onProgress: (progress: ScanProgress) => void,
): Promise<() => void> {
  return listenEvent("library-scan-progress", onProgress);
}

export async function listenExtractProgress(
  onProgress: (progress: ExtractProgress) => void,
): Promise<() => void> {
  return listenEvent("library-extract-progress", onProgress);
}

async function listenEvent<T>(
  event: string,
  onProgress: (payload: T) => void,
): Promise<() => void> {
  try {
    const { listen } = await import("@tauri-apps/api/event");
    return await listen<T>(event, (message) => {
      onProgress(message.payload);
    });
  } catch {
    return () => undefined;
  }
}
