export interface CompanionPaths {
  mangaConExecutable: string;
  mangaConFavoritesJson: string;
  bookshelfRoot: string;
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
  chapterCount: number;
  imageCount: number;
  readProgressPage: number;
  scanStatus: "pending" | "missing" | "matched" | "imported" | "error";
}

export interface BookshelfMatch {
  title: string;
  directory: string;
  chapterCount: number;
  imageCount: number;
}

export interface ImportFavoritesResult {
  imported: number;
  matched: number;
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

export interface LaunchMangaConResult {
  pid: number;
}

export interface AutomationRunStatus {
  state: "waiting_refresh";
  message: string;
  detectedBadges: number;
  stableSamples: number;
}
