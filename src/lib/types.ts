export interface CompanionPaths {
  mangaConExecutable: string;
  mangaConFavoritesJson: string;
  mangaConDatabase: string;
  bookshelfRoot: string;
  databasePath: string;
}

export interface MangaConFavorite {
  id: string;
  name: string;
  location: string;
  tags: string[];
  sourceUri: string;
  sourceScheme?: string;
  sourceDomain?: string;
  localPath?: string;
  coverPath?: string | null;
  chapterCount: number;
  imageCount: number;
  latestChapterTitle?: string | null;
  readProgressPage: number;
  scanStatus: "pending" | "missing" | "matched" | "imported" | "error";
  hasUpdate?: boolean;
}

export interface BookshelfMatch {
  title: string;
  directory: string;
  chapterCount: number;
  imageCount: number;
}

export type ChapterKind = "regular" | "volume" | "machine_translation" | "other";

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

export interface ImportFavoritesResult {
  imported: number;
  matched: number;
  favorites: MangaConFavorite[];
}

export interface SyncBookshelfMatchesResult {
  imported: number;
  scanned: number;
  matched: number;
  missing: number;
  orphaned: number;
  favorites: MangaConFavorite[];
}

export interface MangaConWindow {
  hwnd: number;
  title: string;
}

export interface BadgePoint {
  x: number;
  y: number;
}

export interface WindowPoint {
  x: number;
  y: number;
}

export interface MangaConBadgeScanResult {
  window: MangaConWindow;
  width: number;
  height: number;
  badges: BadgePoint[];
}

export interface OpenFavoritesResult {
  window: MangaConWindow;
  clicked: WindowPoint;
  width: number;
  height: number;
  badges: BadgePoint[];
}

export interface OpenComicResult {
  window: MangaConWindow;
  badge: BadgePoint;
  clicked: WindowPoint;
  width: number;
  height: number;
  remainingBadges: BadgePoint[];
}

export interface OpenScrolledComicResult extends OpenComicResult {
  scrollAttempts: number;
}

export interface DetailUpdateScanResult {
  window: MangaConWindow;
  width: number;
  height: number;
  badges: BadgePoint[];
  scrollAttempts: number;
}

export interface FavoritesUpdateScanPage {
  scrollAttempts: number;
  badges: BadgePoint[];
}

export interface FavoritesUpdateScanResult {
  window: MangaConWindow;
  width: number;
  height: number;
  badges: BadgePoint[];
  pages: FavoritesUpdateScanPage[];
  scrollAttempts: number;
}

export interface TriggerDetailDownloadResult {
  window: MangaConWindow;
  badge: BadgePoint;
  clicked: WindowPoint;
  width: number;
  height: number;
  remainingBadges: BadgePoint[];
  scrollAttempts: number;
}

export type DetailUpdateBatchStoppedReason =
  | "limit_reached"
  | "no_update_badge";

export interface TriggerDetailDownloadBatchResult {
  requestedLimit: number;
  processed: number;
  stoppedReason: DetailUpdateBatchStoppedReason;
  downloads: TriggerDetailDownloadResult[];
}

export interface TriggerNextFavoriteUpdateDownloadResult {
  comic: OpenScrolledComicResult;
  download: TriggerDetailDownloadResult;
  downloadBatch: TriggerDetailDownloadBatchResult;
}

export type FavoriteUpdateSkipReason = "detail_no_update_badge";

export interface SkippedFavoriteUpdateResult {
  comic: OpenScrolledComicResult;
  reason: FavoriteUpdateSkipReason;
}

export type FavoriteUpdateBatchStoppedReason =
  | "limit_reached"
  | "no_update_badge"
  | "detail_no_update_badge";

export interface TriggerFavoriteUpdateBatchResult {
  requestedLimit: number;
  processed: number;
  downloadedChapters: number;
  stoppedReason: FavoriteUpdateBatchStoppedReason;
  skipped: SkippedFavoriteUpdateResult[];
  items: TriggerNextFavoriteUpdateDownloadResult[];
}

export type FavoriteUpdateRecoveryStoppedReason =
  | "completed"
  | "restart_limit_reached";

export type FavoriteUpdateRecoveryEventKind =
  | "started"
  | "run_completed"
  | "comic_downloaded"
  | "comic_skipped"
  | "error"
  | "restarted"
  | "completed"
  | "restart_limit_reached";

export interface FavoriteUpdateRecoveryEvent {
  kind: FavoriteUpdateRecoveryEventKind;
  message: string;
  processed: number;
  downloadedChapters: number;
  skippedCount: number;
  restarts: number;
}

export interface RecoveringFavoriteUpdateResult {
  requestedLimit: number;
  maxRestarts: number;
  restarts: number;
  processed: number;
  downloadedChapters: number;
  skippedCount: number;
  stoppedReason: FavoriteUpdateRecoveryStoppedReason;
  lastError: string | null;
  events: FavoriteUpdateRecoveryEvent[];
  runs: TriggerFavoriteUpdateBatchResult[];
}

export interface QueuedMangaConTask {
  mangaId: number;
  volumeId: number;
  manga: string;
  uri: string;
  volumeKey: string;
  title: string;
  location: string;
  extra: string;
  orderIndex: number;
}

export interface ContinueDownloadConfirmResult {
  found: boolean;
  clicked: boolean;
  dialogTitle: string | null;
}

export interface QueueMangaConUpdatesResult {
  backupPath: string;
  totalUpdates: number;
  queued: number;
  skippedExisting: number;
  clearedUpdateMarkers: number;
  launched: boolean;
  launchPid?: number | null;
  confirm: ContinueDownloadConfirmResult;
  tasks: QueuedMangaConTask[];
}

export interface MangaConTaskStatus {
  totalTasks: number;
  activeTasks: number;
  failedTasks: number;
  finishedTasks: number;
  totalErrors: number;
}

export interface RequeuedMangaConRepairTask {
  taskId: number;
  uri: string;
  volumeKey: string;
  location: string;
  errors: number;
  orderIndex: number;
}

export interface RepairMangaConFailedTasksResult {
  backupPath: string;
  totalFailed: number;
  requeued: number;
  launched: boolean;
  launchPid?: number | null;
  confirm: ContinueDownloadConfirmResult;
  tasks: RequeuedMangaConRepairTask[];
}

export interface LaunchMangaConResult {
  pid: number;
}

export interface EnsureMangaConRunningResult {
  launched: boolean;
  launchPid?: number | null;
  windows: MangaConWindow[];
}

export interface AutomationRunStatus {
  state: "waiting_refresh";
  message: string;
  detectedBadges: number;
  stableSamples: number;
}
