export type ScanStatus = "pending" | "missing" | "matched" | "imported" | "error";
export type ReadingDirection = "ltr" | "rtl";
export type FitMode = "width" | "height" | "contain" | "original";
export type ReadMode = "page" | "scroll" | "spread";
export type ChapterKind = "regular" | "volume" | "machine_translation" | "other";

export interface LibraryPaths {
  bookshelfRoot: string;
  databasePath: string;
  extraRoots?: string[];
}

export interface LibraryComic {
  id: string;
  name: string;
  location: string;
  author?: string | null;
  tags: string[];
  localPath?: string;
  coverPath?: string | null;
  chapterCount: number;
  imageCount: number;
  latestChapterTitle?: string | null;
  readProgressPage: number;
  lastReadChapterId?: string | null;
  lastReadChapterTitle?: string | null;
  lastReadAt?: string | null;
  lastReadChapterOrdinal?: number | null;
  lastReadChapterPages?: number;
  scanStatus: ScanStatus;
  favorited: boolean;
  readingDirection: ReadingDirection;
  fitMode: FitMode;
  readMode?: ReadMode;
  shelfUpdatedAt?: string | null;
  shelfUpdateNote?: string | null;
}

export interface LocalChapter {
  id: string;
  comicId: string;
  title: string;
  path: string;
  ordinal?: number;
  pageCount: number;
  readProgressPage: number;
  specialKind: ChapterKind;
}

export interface LoadLibraryResult {
  databasePath: string;
  bookshelfRoot: string;
  comics: LibraryComic[];
  baselineCompleted?: boolean;
}

export interface ScanLibraryResult {
  scanned: number;
  added: number;
  updated: number;
  unchanged?: number;
  missing: number;
  failed?: number;
  failedItems?: Array<{ title: string; error: string }>;
  databasePath: string;
  bookshelfRoot: string;
  comics: LibraryComic[];
  baselineCompleted?: boolean;
  establishedBaseline?: boolean;
}

export interface ScanProgress {
  scanned: number;
  total: number;
  currentTitle: string;
}

export interface ExtractProgress {
  current: number;
  total: number;
}

export interface CacheStats {
  bytes: number;
  folders: number;
  freedBytes: number;
}

export interface ReaderDefaults {
  readingDirection: ReadingDirection;
  fitMode: FitMode;
  readMode: ReadMode;
}

export interface LibraryViewPrefs {
  filter: "all" | "favorited" | "recent" | "missing";
  sort: "name" | "recent" | "chapters" | "updated";
  sortDesc: boolean;
  query: string;
}

export interface LocalInstallerPackage {
  path: string;
  fileName: string;
  version: string;
  isNewer: boolean;
}

export interface LocalUpdateCheckResult {
  currentVersion: string;
  hasUpdate: boolean;
  latest?: LocalInstallerPackage | null;
  packages: LocalInstallerPackage[];
  searchedDirs: string[];
}
